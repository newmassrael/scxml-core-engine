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
//!   exactly when the field is present ([`Derivation::PresenceOf`]), for the
//!   gated fields a source presents: a gated field that is itself derived (the
//!   length of a gated payload, gated by the same flag) follows the flag and says
//!   nothing back. A gate of any other form (a disjunction, a flag-input, a
//!   chain's membership) names no one flag to set, so it is a [`Check`] the
//!   measure holds a source to.
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

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::forge::codec_totality::child_alias;
use crate::forge::generator::{collect_parent_tag_derivations, ImportContext};
use crate::forge::model::{
    BitSize, CodecEncoding, CodecField, CodecModel, CodecVariant, CountRef, FlagBind,
    FlagBindSource, PresentIfPredicate, PresentIfScope, TlvEntryId, TlvTerminateStrategy,
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
    /// The flag a parent binds to a flag-input of the codecs `binders` hold,
    /// each of which says the input itself (docs/adr/0065 §6 R4a): the input
    /// the body or the arm says. Where none of them is there, the carrier
    /// keeps the bit the value presents.
    BoundInput { binders: Vec<Binder> },
    /// A flag of the disjunction `gates`, which decides a group
    /// (docs/adr/0065 §6 R4c): the value the arm the value holds gives it, or
    /// the one it has while the value holds no arm.
    ArmOf { gates: Vec<GateVar> },
}

/// A body or an arm whose codec says a flag-input the parent binds to one of
/// its flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binder {
    /// The import alias of the codec.
    pub alias: String,
    /// The codec's input the flag is bound to.
    pub input: String,
    pub site: BindSite,
}

/// Where the parent holds a codec it binds an input of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindSite {
    /// An embedded body, the field `field`.
    Embed { field: String },
    /// A list or a chain, the field `field`: one bound value for every entry.
    List { field: String },
    /// The arms of the variant whose body is the codec.
    Arm,
}

impl BindSite {
    /// The element a refusal at this site names: the field, or the alias of
    /// the arm's codec.
    pub fn element<'a>(&'a self, alias: &'a str) -> &'a str {
        match self {
            BindSite::Embed { field } | BindSite::List { field } => field,
            BindSite::Arm => alias,
        }
    }
}

/// How the encode of a codec knows one of its flag-inputs, when the value says
/// it (docs/adr/0065 §6 R4a). The decode is handed every input by the parent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputSource {
    /// The one presented field gated on the input alone: the input is set
    /// exactly when the field is present (absent, for a field gated on
    /// `!input`).
    Gate { field: String, when_clear: bool },
    /// The one embedded body, always there, that the codec hands the input on
    /// to as the input `input` of the codec imported as `alias`, which says it.
    Forward {
        embed: String,
        alias: String,
        input: String,
        /// The body says it in some of its values only (§6 R4e), so this codec
        /// does too.
        partly: bool,
    },
    /// The group the input decides ([`GateGroup`]): the input is set exactly
    /// when the value holds the group's side present while it holds.
    Group,
    /// The variant, every arm of which the codec hands the input on to and
    /// says it (docs/adr/0065 §6 R4d): a value holds one arm, so one thing says
    /// the input in any value. `arms` maps the alias of each arm's codec to the
    /// input of it that is bound.
    EveryArm { arms: BTreeMap<String, String> },
    /// The variant, some arm of which the codec hands the input on to, the
    /// others not reading it (docs/adr/0065 §6 R4e): an arm that says it says it,
    /// and a value in any other arm has no bit the input could change, so nothing
    /// is left for the codec to be handed. `arms` is as above.
    SomeArms { arms: BTreeMap<String, String> },
    /// The one list the codec hands the input on to, every entry of which says
    /// it (docs/adr/0065 §6 R8): the list's start is the input, empty or not.
    /// The list `list` holds entries of the codec imported as `alias`, whose
    /// input `input` is bound; a gated list says nothing while it is absent.
    List {
        list: String,
        alias: String,
        input: String,
        partly: bool,
    },
}

impl InputSource {
    /// Whether a value can say nothing of the input, so that reading it off a
    /// source answers an `Option` (docs/adr/0065 §6 R4e).
    pub fn is_partial(&self) -> bool {
        matches!(
            self,
            InputSource::SomeArms { .. }
                | InputSource::Forward { partly: true, .. }
                | InputSource::List { partly: true, .. }
        )
    }
}

/// What a gate reads: a flag-input of the codec, a one-bit flag of one of its
/// own carriers, or whether a chain of it holds an entry of a kind
/// (docs/adr/0065 §6 R6), `bare` where the kind's identifier is all an entry of
/// it is, so that the codec derives the entry and the value holds its place.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GateVar {
    Input(String),
    Flag { carrier: String, flag: String },
    Member { chain: String, key: u64, bare: bool },
}

impl fmt::Display for GateVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GateVar::Input(name) => f.write_str(name),
            GateVar::Flag { carrier, flag } => write!(f, "{carrier}.{flag}"),
            GateVar::Member { chain, key, .. } => write!(f, "{chain}.has(0x{key:X})"),
        }
    }
}

/// The elements a value presents together because the same gates decide them
/// (docs/adr/0065 §6 R4b, R4c): a sum of arms, each the elements present under
/// one assignment of the gates, and possibly no arm at all. A value holds one
/// arm, or none where `absent` says the gates allow that, and the gates are
/// derived from which.
///
/// One flag that gates elements on both of its values makes two arms, one for
/// each value. One that gates elements on one value makes one arm, which the
/// value holds or not. A disjunction makes an arm for each assignment that
/// satisfies it, each holding every element it gates, and no arm under the one
/// that does not. A length stated while its content is absent is in the arm
/// where the content is not. A gate with one element keeps the element as it
/// is; a group has two elements or more, or more than one arm.
///
/// The gates on a chain's entries, and the flags joined with them, make an arm
/// for every assignment of them (R6), each holding the chain refined by the
/// memberships it sets and clears, so a value always holds one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateGroup {
    /// The flags that decide the group, in the order its gate names them.
    pub gates: Vec<GateVar>,
    /// The arms, in the order a type of the group lists them.
    pub arms: Vec<GroupArm>,
    /// The values of the gates while the value holds no arm, when it may hold
    /// none.
    pub absent: Option<Vec<bool>>,
    /// The list always present and the list one gate decides, when they share
    /// one count (docs/adr/0065 §6 R5): the first is in every arm, and an arm
    /// that holds the second holds both as one value of one count.
    pub aligned: Option<(String, String)>,
}

/// One arm of a [`GateGroup`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupArm {
    /// The value of each gate while the value holds the arm, in the order of
    /// [`GateGroup::gates`].
    pub values: Vec<bool>,
    /// The elements present in it, in declaration order.
    pub fields: Vec<String>,
}

impl GateGroup {
    /// Whether the group has more than one arm, so that a value says which.
    pub fn is_sum(&self) -> bool {
        self.arms.len() > 1
    }

    /// The list `field` shares its count with in `arm`, when `field` is the
    /// first of the two lists of one count and the arm holds the second
    /// (docs/adr/0065 §6 R5).
    pub fn second_of(&self, arm: &GroupArm, field: &str) -> Option<&str> {
        self.aligned
            .as_ref()
            .filter(|(first, second)| first == field && arm.fields.contains(second))
            .map(|(_, second)| second.as_str())
    }

    /// Whether `field` is the second of the two lists of one count in `arm`,
    /// held with the first ([`GateGroup::second_of`]).
    pub fn is_second_in(&self, arm: &GroupArm, field: &str) -> bool {
        self.aligned
            .as_ref()
            .is_some_and(|(_, second)| second == field && arm.fields.iter().any(|f| f == field))
    }

    /// The chain whose memberships are gates of the group, when some are
    /// (docs/adr/0065 §6 R6a, R6b). Such a group holds the chain in each arm,
    /// refined by the memberships the arm sets and clears.
    pub fn chain(&self) -> Option<&str> {
        self.gates.iter().find_map(|gate| match gate {
            GateVar::Member { chain, .. } => Some(chain.as_str()),
            _ => None,
        })
    }

    /// The memberships among the group's gates, each as its place among them
    /// and the kind of entry it asks for, in the order of [`GateGroup::gates`].
    pub fn members(&self) -> impl Iterator<Item = (usize, u64)> + '_ {
        self.gates
            .iter()
            .enumerate()
            .filter_map(|(index, gate)| match gate {
                GateVar::Member { key, .. } => Some((index, *key)),
                _ => None,
            })
    }

    /// The kinds of entry the chain holds an entry of in `arm`, and those it
    /// holds none of, each in the order of [`GateGroup::gates`].
    pub fn kinds_in(&self, arm: &GroupArm) -> (Vec<u64>, Vec<u64>) {
        let (held, not): (Vec<_>, Vec<_>) =
            self.members().partition(|(index, _)| arm.values[*index]);
        (
            held.into_iter().map(|(_, key)| key).collect(),
            not.into_iter().map(|(_, key)| key).collect(),
        )
    }

    /// Whether the group's one gate is a chain's membership of a bare entry
    /// (docs/adr/0065 §6 R6a), whose arm where it holds holds the entry's
    /// place, not the entry. A bare entry among other gates is held where it
    /// stands, as an entry with content is (R6b): one place says where the
    /// first entry of one kind is, and the chain whole says it of every kind.
    pub fn member_is_bare(&self) -> bool {
        matches!(self.gates.as_slice(), [GateVar::Member { bare: true, .. }])
    }

    /// Whether `field` is one of the group's elements.
    pub fn contains(&self, field: &str) -> bool {
        self.arms
            .iter()
            .any(|a| a.fields.iter().any(|f| f == field))
    }

    /// The group's elements, each once, in the order the arms first name them.
    pub fn fields(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for arm in &self.arms {
            for field in &arm.fields {
                if !out.contains(&field.as_str()) {
                    out.push(field);
                }
            }
        }
        out
    }

    /// The group of one gate made of the elements present while it holds and
    /// those present while it does not.
    fn of_one(gate: GateVar, set: Vec<String>, clear: Vec<String>) -> GateGroup {
        let arm = |value: bool, fields: Vec<String>| GroupArm {
            values: vec![value],
            fields,
        };
        let (arms, absent) = match (set.is_empty(), clear.is_empty()) {
            (false, false) => (vec![arm(true, set), arm(false, clear)], None),
            (false, true) => (vec![arm(true, set)], Some(vec![false])),
            _ => (vec![arm(false, clear)], Some(vec![true])),
        };
        GateGroup {
            gates: vec![gate],
            arms,
            absent,
            aligned: None,
        }
    }
}

/// A flag-input of a held codec that the codec says itself, while the parent
/// hands it a value from elsewhere: a flag bound to every entry of a list, an
/// input the parent is given, an input a chain supplies from the entry before.
/// The encode holds each to the input the codec says (docs/adr/0065 §6 R4a).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputCheck {
    pub binder: Binder,
    pub against: InputValue,
}

/// The value a parent hands a held codec's flag-input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputValue {
    /// A flag of the parent's own carrier, as the value presents it.
    Carrier { carrier: String, flag: String },
    /// The parent's own flag-input.
    Input { name: String },
    /// 1 for an entry whose entry before has the identifier `id`, 0 for every
    /// other entry and the first (`<sce:entry-flag-bind>`).
    PreviousEntry { id: u64 },
}

impl fmt::Display for InputValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputValue::Carrier { carrier, flag } => write!(f, "`{carrier}.{flag}`"),
            InputValue::Input { name } => write!(f, "the input `{name}`"),
            InputValue::PreviousEntry { id } => {
                write!(f, "whether the entry before has the identifier 0x{id:X}")
            }
        }
    }
}

/// What the inputs of a codec depend on in the codecs it imports.
pub trait InputImports {
    /// The binds the import `alias` declares (`<sce:flag-bind>`).
    fn flag_binds(&mut self, alias: &str) -> Vec<FlagBind>;
    /// Whether the codec imported as `alias` says its input `input` itself
    /// ([`CodecRoles::inputs`]).
    fn says_input(&mut self, alias: &str, input: &str) -> bool;
    /// Whether it says `input` in some of its values only
    /// ([`InputSource::is_partial`]).
    fn says_input_partly(&mut self, alias: &str, input: &str) -> bool;
    /// What the codec imported as `alias` is made of ([`EntryShape`]), or
    /// `None` when it is no codec this one can read.
    fn entry_shape(&mut self, alias: &str) -> Option<EntryShape>;
}

/// What a codec held as a chain's entry is made of: enough to tell whether the
/// entry a gate on the chain names holds anything but its identifier
/// ([`named_entry_is_bare`], docs/adr/0065 §6 R6).
#[derive(Debug, Clone, Default)]
pub struct EntryShape {
    /// The codec's fields.
    pub fields: Vec<CodecField>,
    /// The codec's variant, when it has one.
    pub variant: Option<CodecVariant>,
    /// The values of the variant's arms whose body codec presents nothing: no
    /// field and no variant of its own.
    pub empty_arm_tags: Vec<u64>,
}

/// Whether the identifier `id` of a chain's entries can hold the key `key`: a
/// key wider than the flag or field it is read from, or with a bit the
/// identifier leaves out, names no entry, and a gate on it is refused where the
/// import is resolved rather than held as a group (docs/adr/0065 §6 R6).
pub fn named_key_fits(shape: &EntryShape, id: &TlvEntryId, key: u64) -> bool {
    let ones = |width: u32| {
        if width >= 64 {
            u64::MAX
        } else {
            (1u64 << width) - 1
        }
    };
    match &id.carrier {
        Some(carrier) => shape
            .fields
            .iter()
            .find(|f| &f.id == carrier)
            .and_then(|f| f.flags.iter().find(|d| d.name == id.name))
            .is_some_and(|flag| key & !ones(flag.width) == 0),
        None => {
            let Some(field) = shape.fields.iter().find(|f| f.id == id.name) else {
                return false;
            };
            let BitSize::Fixed { bits } = field.bit_size else {
                return false;
            };
            let left_out = field
                .flags
                .iter()
                .filter(|f| id.except.contains(&f.name))
                .fold(0u64, |mask, f| {
                    mask | ones(f.width).checked_shl(f.bit).unwrap_or(0)
                });
            key & !ones(bits) == 0 && key & left_out == 0
        }
    }
}

/// Whether the entry of the kind `key`, told apart by the identifier `id` of a
/// chain that ends on the flag `chain_flag`, holds nothing but its identifier,
/// so that the identifier is all its value is (docs/adr/0065 §6 R6, R6a).
///
/// That is so when the identifier is a whole flags carrier, the flags it leaves
/// out are the one the chain writes by an entry's place, the carrier is the
/// entry's one field, and the entry has no variant or one whose tag is a flag
/// the identifier holds and whose arm for the key's tag presents nothing. An
/// identifier that is one flag of the carrier, or a field beside others, leaves
/// the rest of the entry to its content.
pub fn named_entry_is_bare(
    shape: &EntryShape,
    id: &TlvEntryId,
    chain_flag: Option<&str>,
    key: u64,
) -> bool {
    if id.carrier.is_some() || shape.fields.len() != 1 {
        return false;
    }
    let carrier = &shape.fields[0];
    if carrier.id != id.name || carrier.flags.is_empty() {
        return false;
    }
    if !id.except.iter().all(|e| Some(e.as_str()) == chain_flag) {
        return false;
    }
    if !named_key_fits(shape, id, key) {
        return false;
    }
    let Some(variant) = &shape.variant else {
        return true;
    };
    let Some(tag_flag) = variant
        .tag_flag
        .as_deref()
        .filter(|_| variant.tag_field.as_deref() == Some(carrier.id.as_str()))
        .filter(|t| !id.except.iter().any(|e| e == t))
        .and_then(|t| carrier.flags.iter().find(|f| f.name == t))
    else {
        return false;
    };
    let width = tag_flag.width;
    let mask = if width >= 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    };
    let tag = key.checked_shr(tag_flag.bit).unwrap_or(0) & mask;
    variant
        .arms
        .iter()
        .any(|arm| arm.value == tag && shape.empty_arm_tags.contains(&arm.value))
}

/// The memberships of `model`'s chains a clause of a gate asks for, each chain
/// and key whose identifier holds the key (`named_key_fits`), with whether the
/// named entry holds nothing but its identifier ([`named_entry_is_bare`]). Each
/// is an atom of the groups of its chain ([`membership_groups`], docs/adr/0065
/// §6 R6a, R6b).
fn memberships(model: &CodecModel, inputs: &mut dyn InputImports) -> BTreeMap<(String, u64), bool> {
    let mut held = BTreeMap::new();
    let clauses = model.fields.iter().flat_map(|field| {
        std::iter::successors(field.present_if.as_ref(), |p| p.or_with.as_deref())
    });
    for gate in clauses {
        let (PresentIfScope::Chain, Some(key)) = (gate.scope, gate.entry_id) else {
            continue;
        };
        let Some(chain) = model.fields.iter().find(|c| c.id == gate.field_id) else {
            continue;
        };
        let BitSize::TlvChain {
            entry_id: Some(id),
            terminate_on,
            ..
        } = &chain.bit_size
        else {
            continue;
        };
        let Some(alias) = chain.tlv_chain_body_alias.as_deref() else {
            continue;
        };
        let chain_flag = match terminate_on {
            TlvTerminateStrategy::EntryFlag { flag_name } => Some(flag_name.as_str()),
            TlvTerminateStrategy::ExhaustOrDepth => None,
        };
        let Some(shape) = inputs.entry_shape(alias) else {
            continue;
        };
        if !named_key_fits(&shape, id, key) {
            continue;
        }
        let bare = named_entry_is_bare(&shape, id, chain_flag, key);
        held.insert((chain.id.clone(), key), bare);
    }
    held
}

/// The imports a codec's generation resolved, each with the inputs it says
/// ([`ImportContext::codec_says_inputs`]).
pub struct Contexts<'a>(pub &'a [ImportContext]);

impl InputImports for Contexts<'_> {
    fn flag_binds(&mut self, alias: &str) -> Vec<FlagBind> {
        self.0
            .iter()
            .find(|i| i.alias == alias)
            .map(|i| i.flag_binds.clone())
            .unwrap_or_default()
    }

    fn says_input(&mut self, alias: &str, input: &str) -> bool {
        self.0
            .iter()
            .any(|i| i.alias == alias && i.codec_says_inputs.iter().any(|s| s == input))
    }

    fn says_input_partly(&mut self, alias: &str, input: &str) -> bool {
        self.0
            .iter()
            .any(|i| i.alias == alias && i.codec_partly_says_inputs.iter().any(|s| s == input))
    }

    fn entry_shape(&mut self, alias: &str) -> Option<EntryShape> {
        self.0
            .iter()
            .find(|i| i.alias == alias && i.kind == "codec")
            .map(|i| EntryShape {
                fields: i.codec_fields.clone(),
                variant: i.codec_variant.clone(),
                empty_arm_tags: i.codec_empty_arm_tags.clone(),
            })
    }
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
    /// The flag-input the gate is, when it is one input alone (`I` or `!I`).
    pub input: Option<String>,
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
            Derivation::VariantTag { .. } | Derivation::ArmOf { .. } => 1,
            // The arms of one variant are one source: one of them is there.
            Derivation::BoundInput { binders } => {
                let embeds = binders
                    .iter()
                    .filter(|b| !matches!(b.site, BindSite::Arm))
                    .count();
                embeds + usize::from(binders.iter().any(|b| matches!(b.site, BindSite::Arm)))
            }
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
            (Derivation::BoundInput { binders }, Derivation::BoundInput { binders: more }) => {
                binders.extend(more)
            }
            // One tag per codec: a second is the same declaration read twice.
            (Derivation::VariantTag { .. }, Derivation::VariantTag { .. }) => {}
            // A flag is read by one disjunction.
            (Derivation::ArmOf { .. }, Derivation::ArmOf { .. }) => {}
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
            Derivation::BoundInput { binders } => {
                let each: Vec<String> = binders
                    .iter()
                    .map(|b| format!("{}.{}", b.site.element(&b.alias), b.input))
                    .collect();
                write!(f, "input-of({})", each.join(", "))
            }
            Derivation::ArmOf { gates } => write!(f, "arm-of({})", spell_gates(gates)),
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
    /// A gate a said input or a disjunction's group decides stays here for an
    /// encode that holds neither; `source_checks` leaves it out.
    pub checks: Vec<Check>,
    /// The flag-inputs the value says, by name, and what says each
    /// (docs/adr/0065 §6 R4a). The encode of a source takes the others.
    pub inputs: BTreeMap<String, InputSource>,
    /// The inputs of held codecs that a codec says while the parent hands it a
    /// value from elsewhere: each a relation the encode holds a source to.
    pub input_checks: Vec<InputCheck>,
    /// The inputs the entries of a list say while the parent hands every entry
    /// one value, which the list holds as its law (docs/adr/0065 §6 R8): the
    /// value is the list's start, and nothing is left for the encode to hold.
    /// Each names the list by its site.
    pub list_laws: Vec<InputCheck>,
    /// The elements one gate decides, which a value presents together
    /// (docs/adr/0065 §6 R4b).
    pub groups: Vec<GateGroup>,
    /// A derivation that names an element the codec does not have. A codec the
    /// parser accepted has none; a non-empty list is a defect of the model.
    pub unresolved: Vec<String>,
}

impl CodecRoles {
    /// The gates a source is held to: the checks, less those on an input the
    /// value says, which the encode derives instead. Every check holds for an
    /// encode that is handed its inputs, as another backend's is.
    pub fn source_checks(&self) -> impl Iterator<Item = &Check> {
        self.checks.iter().filter(|check| {
            check
                .input
                .as_ref()
                .is_none_or(|input| !self.inputs.contains_key(input))
                // A field of a disjunction's group is held with the arm that
                // says its flags (docs/adr/0065 §6 R4c), and one of a chain's
                // memberships with the arm that says the chain (R6a, R6b).
                && !self
                    .group_of(&check.field)
                    .is_some_and(|group| group.gates.len() > 1 || group.chain().is_some())
        })
    }

    /// Whether the value says the flag-input `name`.
    pub fn says_input(&self, name: &str) -> bool {
        self.inputs.contains_key(name)
    }

    /// Whether the value says the flag-input `name` in some of its values only.
    pub fn says_input_partly(&self, name: &str) -> bool {
        self.inputs.get(name).is_some_and(InputSource::is_partial)
    }

    /// The group `field` is an element of.
    pub fn group_of(&self, field: &str) -> Option<&GateGroup> {
        self.groups.iter().find(|g| g.contains(field))
    }

    /// Whether `list` says the count it shares through the list a group holds
    /// it with, so that it is no second source of the number (docs/adr/0065 §6
    /// R5).
    pub fn counted_with_another(&self, list: &str) -> bool {
        self.groups
            .iter()
            .filter_map(|g| g.aligned.as_ref())
            .any(|(_, second)| second == list)
    }

    /// The group the gate `gate` decides.
    pub fn group_of_gate(&self, gate: &GateVar) -> Option<&GateGroup> {
        self.groups
            .iter()
            .find(|g| g.gates.len() == 1 && &g.gates[0] == gate)
    }

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
        for check in self.source_checks() {
            out.push_str(&format!(
                "  check  `{}` is present exactly when {}\n",
                check.field, check.gate
            ));
        }
        for (name, source) in &self.inputs {
            let by = match source {
                InputSource::Gate {
                    field,
                    when_clear: false,
                } => format!("presence-of({field})"),
                InputSource::Gate {
                    field,
                    when_clear: true,
                } => format!("absence-of({field})"),
                InputSource::Forward { embed, input, .. } => format!("input-of({embed}.{input})"),
                InputSource::Group => format!("side-of(group {name})"),
                InputSource::EveryArm { .. } => "input-of(every arm)".to_string(),
                InputSource::SomeArms { .. } => "input-of(the arms that read it)".to_string(),
                InputSource::List { list, input, .. } => format!("start-of({list}.{input})"),
            };
            out.push_str(&format!("  input  {name}  said by the value  {by}\n"));
        }
        for group in &self.groups {
            let value = |values: &[bool]| {
                values
                    .iter()
                    .map(|v| if *v { "1" } else { "0" })
                    .collect::<String>()
            };
            let mut arms: Vec<String> = group
                .arms
                .iter()
                .map(|arm| format!("{} ({})", value(&arm.values), arm.fields.join(", ")))
                .collect();
            if let Some(absent) = &group.absent {
                arms.push(format!("{} (none)", value(absent)));
            }
            if let Some((first, second)) = &group.aligned {
                arms.push(format!("one count ({first}, {second})"));
            }
            out.push_str(&format!(
                "  group  {}  {}\n",
                spell_gates(&group.gates),
                arms.join("  ")
            ));
        }
        for check in &self.input_checks {
            out.push_str(&format!(
                "  check  `{}` says its input `{}` as {} does\n",
                check.binder.site.element(&check.binder.alias),
                check.binder.input,
                check.against
            ));
        }
        for reference in &self.unresolved {
            out.push_str(&format!("  UNRESOLVED  {reference}\n"));
        }
        out
    }
}

/// The gates of a group as the table spells them: a disjunction's joined by
/// `||` (docs/adr/0065 §6 R4c), the atoms of a chain's memberships, decided
/// together and joined by no operator, listed (R6b).
fn spell_gates(gates: &[GateVar]) -> String {
    let each: Vec<String> = gates.iter().map(GateVar::to_string).collect();
    if gates.iter().any(|g| matches!(g, GateVar::Member { .. })) {
        each.join(", ")
    } else {
        each.join(" || ")
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
/// list ([`ImportContext`]), which the parent-tag carriers and the inputs of
/// held codecs are read from; a codec that imports nothing takes `&[]`.
pub fn classify(model: &CodecModel, imports: &[ImportContext]) -> CodecRoles {
    classify_with(model, imports, &mut Contexts(imports))
}

/// [`classify`], with the inputs of held codecs read from `inputs`: an
/// analysis that has no resolved imports still knows which inputs the codecs
/// it holds say.
pub fn classify_with(
    model: &CodecModel,
    imports: &[ImportContext],
    inputs: &mut dyn InputImports,
) -> CodecRoles {
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
        // a relation the measure checks. So is a gate on a flag wider than one
        // bit: presence says whether it is zero, not which value it holds.
        if let Some(gate) = &field.present_if {
            let one_bit = model
                .fields
                .iter()
                .find(|c| c.id == gate.field_id)
                .and_then(|c| c.flags.iter().find(|d| d.name == gate.flag_name))
                .is_none_or(|d| d.width == 1);
            if gate.scope == PresentIfScope::Local && gate.or_with.is_none() && one_bit {
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
                    input: (gate.scope == PresentIfScope::Input && gate.or_with.is_none())
                        .then(|| gate.flag_name.clone()),
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

    // A flag of this codec bound to an input that the codec of a body or an
    // arm says is derived from it, as a parent tag is. A list hands its one
    // bound value to every entry, which the list holds as its law and its start
    // says (docs/adr/0065 §6 R8), entries that say nothing of it included (R8b).
    // A body that says it in some of its values only is checked instead, as is a
    // chain whose memberships an arm refines: the flag is the value's where it
    // says nothing.
    let held = held_codecs(model, inputs);
    let members = memberships(model, inputs);
    let lawful = |field: &str| !members.keys().any(|(chain, _)| chain == field);
    let mut input_checks: Vec<InputCheck> = Vec::new();
    let mut list_laws: Vec<InputCheck> = Vec::new();
    for held_codec in &held {
        for bind in &held_codec.binds {
            let FlagBindSource::Carrier { carrier, flag } = &bind.source else {
                continue;
            };
            if !held_codec.says(&bind.input) {
                continue;
            }
            for site in &held_codec.sites {
                let binder = Binder {
                    alias: held_codec.alias.clone(),
                    input: bind.input.clone(),
                    site: site.clone(),
                };
                let against = InputValue::Carrier {
                    carrier: carrier.clone(),
                    flag: flag.clone(),
                };
                let partly = held_codec.says_partly(&bind.input);
                let derived = match site {
                    BindSite::Embed { .. } | BindSite::Arm => !partly,
                    BindSite::List { field } => lawful(field),
                };
                if !derived {
                    input_checks.push(InputCheck { binder, against });
                    continue;
                }
                if matches!(site, BindSite::List { .. }) {
                    list_laws.push(InputCheck {
                        binder: binder.clone(),
                        against,
                    });
                }
                derive(
                    ElementId::Flag {
                        field: carrier.clone(),
                        flag: flag.clone(),
                    },
                    Derivation::BoundInput {
                        binders: vec![binder],
                    },
                )
            }
        }
    }

    // A field derived whole is announced by the flag it is gated on, and says
    // nothing back: its presence follows the flag, so it cannot be what the flag
    // is derived from. The flag is derived from the gated fields that are
    // presented; a flag that gates none is presented.
    let derived_whole: BTreeSet<String> = derivations
        .iter()
        .filter_map(|(id, by)| match id {
            ElementId::Field(field) if !by.is_empty() => Some(field.clone()),
            _ => None,
        })
        .collect();
    checks.retain(|check| !derived_whole.contains(&check.field));
    derivations.retain(|_, by| {
        for derivation in by.iter_mut() {
            if let Derivation::PresenceOf { gated } = derivation {
                gated.retain(|g| !derived_whole.contains(&g.field));
            }
        }
        by.retain(|d| !matches!(d, Derivation::PresenceOf { gated } if gated.is_empty()));
        !by.is_empty()
    });

    // A flag a declaration of another kind derives is not also the input a
    // held codec says: the codec is held to the flag instead.
    for (id, by) in derivations.iter_mut() {
        if by.len() < 2 {
            continue;
        }
        let ElementId::Flag { field, flag } = id else {
            continue;
        };
        by.retain(|d| match d {
            Derivation::BoundInput { binders } => {
                // A list held to the flag has no law of its own: its entries are
                // checked against the flag the other declaration gives it.
                list_laws.retain(|law| !binders.contains(&law.binder));
                input_checks.extend(binders.iter().map(|binder| InputCheck {
                    binder: binder.clone(),
                    against: InputValue::Carrier {
                        carrier: field.clone(),
                        flag: flag.clone(),
                    },
                }));
                false
            }
            _ => true,
        });
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
    // The groups read what is derived (a stated length is one), so they and
    // what depends on them are found once the roles are.
    let mut roles = CodecRoles {
        codec: model.name.clone(),
        encoding: model.encoding,
        rows,
        checks,
        inputs: BTreeMap::new(),
        input_checks: Vec::new(),
        list_laws: Vec::new(),
        groups: Vec::new(),
        unresolved,
    };
    roles.groups = gate_groups(model, &roles, &derived_whole, &members);
    // The flags of a disjunction that decides a group are derived from the arm
    // the value holds (docs/adr/0065 §6 R4c).
    for group in roles.groups.iter().filter(|g| g.gates.len() > 1) {
        for gate in &group.gates {
            let GateVar::Flag { carrier, flag } = gate else {
                continue;
            };
            let id = ElementId::Flag {
                field: carrier.clone(),
                flag: flag.clone(),
            };
            if let Some(row) = roles.rows.iter_mut().find(|row| row.id == id) {
                row.role = Role::Derived(vec![Derivation::ArmOf {
                    gates: group.gates.clone(),
                }]);
            }
        }
    }
    let inputs_said = said_inputs(model, &held, &derived_whole, &roles.groups, &lawful);
    for held_codec in &held {
        // An input of this codec handed on: checked where the value says it
        // otherwise than the body it is handed to. A list that says it holds it
        // as its law (docs/adr/0065 §6 R8).
        for bind in &held_codec.binds {
            let FlagBindSource::Input { name } = &bind.source else {
                continue;
            };
            if !held_codec.says(&bind.input) {
                continue;
            }
            for site in &held_codec.sites {
                let derived_from_it = match (inputs_said.get(name), site) {
                    (Some(InputSource::Forward { embed, .. }), BindSite::Embed { field }) => {
                        embed == field
                    }
                    (
                        Some(InputSource::EveryArm { .. } | InputSource::SomeArms { .. }),
                        BindSite::Arm,
                    ) => true,
                    (Some(InputSource::List { list, .. }), BindSite::List { field }) => {
                        list == field
                    }
                    _ => false,
                };
                let check = InputCheck {
                    binder: Binder {
                        alias: held_codec.alias.clone(),
                        input: bind.input.clone(),
                        site: site.clone(),
                    },
                    against: InputValue::Input { name: name.clone() },
                };
                match site {
                    BindSite::List { .. } if derived_from_it => list_laws.push(check),
                    _ if derived_from_it => {}
                    _ => input_checks.push(check),
                }
            }
        }
    }
    // An input a chain supplies from the entry before, which the entry says: the
    // chain holds it as its law (docs/adr/0065 §6 R8b), and a chain whose
    // memberships an arm refines is checked instead.
    for field in &model.fields {
        let BitSize::TlvChain { entry_binds, .. } = &field.bit_size else {
            continue;
        };
        let Some(held_codec) = held
            .iter()
            .find(|h| Some(h.alias.as_str()) == child_alias(field))
        else {
            continue;
        };
        for bind in entry_binds {
            if held_codec.says(&bind.input) {
                let law = InputCheck {
                    binder: Binder {
                        alias: held_codec.alias.clone(),
                        input: bind.input.clone(),
                        site: BindSite::List {
                            field: field.id.clone(),
                        },
                    },
                    against: InputValue::PreviousEntry {
                        id: bind.previous_entry_id,
                    },
                };
                if lawful(&field.id) {
                    list_laws.push(law);
                } else {
                    input_checks.push(law);
                }
            }
        }
    }

    roles.inputs = inputs_said;
    roles.input_checks = input_checks;
    roles.list_laws = list_laws;
    roles
}

/// The groups of `model` (docs/adr/0065 §6 R4b, R4c): the presented fields
/// each gate of one flag decides, by its value, with a stated length where its
/// content is absent; and the presented fields one disjunction of the codec's
/// own flags decides, an arm for each assignment that satisfies it. A gate of
/// one flag that decides one element makes no group.
fn gate_groups(
    model: &CodecModel,
    roles: &CodecRoles,
    derived_whole: &BTreeSet<String>,
    held: &BTreeMap<(String, u64), bool>,
) -> Vec<GateGroup> {
    // The gates on a chain's entries and what they reach are decided together
    // (docs/adr/0065 §6 R6), so the gates of one flag and the disjunctions
    // decide the rest.
    let members = membership_groups(model, roles, derived_whole, held);
    let joined: BTreeSet<String> = members
        .iter()
        .flat_map(|g| g.fields().into_iter().map(str::to_string))
        .collect();
    // The fields one flag decides, by its value: present while it holds, and
    // while it does not.
    let mut sides: Vec<(GateVar, Vec<String>, Vec<String>)> = Vec::new();
    for field in &model.fields {
        let Some(gate) = &field.present_if else {
            continue;
        };
        if derived_whole.contains(&field.id) || joined.contains(&field.id) {
            continue;
        }
        let Some(var) =
            gate_var(model, gate, held).filter(|var| !matches!(var, GateVar::Member { .. }))
        else {
            continue;
        };
        let index = match sides.iter().position(|(g, _, _)| *g == var) {
            Some(index) => index,
            None => {
                sides.push((var, Vec::new(), Vec::new()));
                sides.len() - 1
            }
        };
        if gate.negate {
            sides[index].2.push(field.id.clone());
        } else {
            sides[index].1.push(field.id.clone());
        }
    }
    // A length stated while its content is absent is on the other side from
    // all of its content.
    let side_of = |sides: &[(GateVar, Vec<String>, Vec<String>)], field: &str| {
        sides
            .iter()
            .enumerate()
            .find_map(|(index, (_, set, clear))| {
                if set.iter().any(|f| f == field) {
                    Some((index, true))
                } else if clear.iter().any(|f| f == field) {
                    Some((index, false))
                } else {
                    None
                }
            })
    };
    let stated: Vec<(String, usize, bool)> = model
        .fields
        .iter()
        .filter(|f| crate::forge::codec_totality::stated(model, roles, f))
        .filter_map(|f| {
            let measured = crate::forge::codec_totality::measured(
                model,
                roles,
                &ElementId::Field(f.id.clone()),
            );
            let (index, side) = side_of(&sides, &measured.first()?.id)?;
            measured
                .iter()
                .all(|m| side_of(&sides, &m.id) == Some((index, side)))
                .then(|| (f.id.clone(), index, !side))
        })
        .collect();
    for (field, index, side) in stated {
        if side {
            sides[index].1.push(field);
        } else {
            sides[index].2.push(field);
        }
    }
    // A list one gate decides that shares its count with a list always present
    // is on the side where the gate holds with that list, which is on both
    // sides: the count is the one list's, and the other has no count of its own
    // to disagree with it (docs/adr/0065 §6 R5).
    let mut aligned: Vec<(usize, String, String)> = Vec::new();
    for (index, (_, set, clear)) in sides.iter_mut().enumerate() {
        let shared = set
            .iter()
            .chain(clear.iter())
            .find_map(|gated| Some((gated.clone(), one_count_list(model, gated)?)));
        if let Some((second, first)) = shared {
            set.insert(0, first.clone());
            clear.insert(0, first.clone());
            aligned.push((index, first, second));
        }
    }
    let mut groups: Vec<GateGroup> = sides
        .into_iter()
        .enumerate()
        .filter(|(_, (_, set, clear))| set.len() + clear.len() >= 2)
        .map(|(index, (gate, set, clear))| GateGroup {
            aligned: aligned
                .iter()
                .find(|(side, _, _)| *side == index)
                .map(|(_, first, second)| (first.clone(), second.clone())),
            ..GateGroup::of_one(gate, set, clear)
        })
        .collect();
    groups.extend(members);
    groups.extend(disjunction_groups(model, roles, derived_whole, held));
    groups
}

/// The list always present that the gated list `gated` shares its count with,
/// when they are the only two lists of that count (docs/adr/0065 §6 R5). Three
/// lists of one count, or two that are both gated, stay as they are: the one
/// number is then a check.
fn one_count_list(model: &CodecModel, gated: &str) -> Option<String> {
    let count_of = |f: &CodecField| match &f.bit_size {
        BitSize::Repeat {
            count_ref: CountRef::LengthField(count),
        } => Some(count.clone()),
        _ => None,
    };
    let list = model.fields.iter().find(|f| f.id == gated)?;
    let count = count_of(list)?;
    let sharing: Vec<&CodecField> = model
        .fields
        .iter()
        .filter(|f| count_of(f).as_ref() == Some(&count))
        .collect();
    match sharing.as_slice() {
        [first, second] if first.present_if.is_none() && second.id == list.id => {
            Some(first.id.clone())
        }
        _ => None,
    }
}

/// The atoms the gate `gate` reads, each with whether its clause is negated, in
/// the order its clauses name them, and whether every clause reads one
/// ([`gate_var`]).
fn gate_atoms(
    model: &CodecModel,
    gate: &PresentIfPredicate,
    held: &BTreeMap<(String, u64), bool>,
) -> (Vec<(GateVar, bool)>, bool) {
    let mut atoms = Vec::new();
    let mut whole = true;
    for clause in std::iter::successors(Some(gate), |p| p.or_with.as_deref()) {
        let single = PresentIfPredicate {
            or_with: None,
            ..clause.clone()
        };
        match gate_var(model, &single, held) {
            Some(var) => atoms.push((var, clause.negate)),
            None => whole = false,
        }
    }
    (atoms, whole)
}

/// Whether the gate made of `clauses` holds under `values`, the value of each
/// of `gates`.
fn gate_holds(clauses: &[(GateVar, bool)], gates: &[GateVar], values: &[bool]) -> bool {
    clauses.iter().any(|(var, negate)| {
        let at = gates
            .iter()
            .position(|g| g == var)
            .expect("a clause reads a gate of its group");
        values[at] != *negate
    })
}

/// The most atoms a group of a chain's memberships is made of: a value holds one
/// arm for each assignment of them, 256 at this bound, and the runtime's sets of
/// kinds (`Kinds`) go up to this many.
const MOST_GROUP_ATOMS: usize = 8;

/// The groups of the gates on `model`'s chains' entries (docs/adr/0065 §6 R6a,
/// R6b). The fields a gate on a chain's entries decides are present by the
/// value of the gates their own gates read: the chain's memberships, and the
/// one-bit flags of the codec's own carriers a gate joins with one (`a.has(k)
/// || h.W`). Each such flag reaches the fields of every gate that reads it, and
/// each membership the chain of every other, so the atoms of one chain are the
/// least set closed under both, and the fields are those whose gates read one.
///
/// They make one group with an arm for each assignment of the atoms, each
/// holding the chain, refined by the memberships it sets and clears, and the
/// fields whose gates hold under it, with a stated length where its content is
/// absent; the flags are derived from the arm (R4c). Every arm holds the chain,
/// so a value always holds one. The arms are listed by how many atoms they set,
/// the most first, then in the order the gates name the atoms, so the arm where
/// one membership holds comes before the one where it does not.
///
/// The atoms of one group are of one chain, and at most [`MOST_GROUP_ATOMS`]; a
/// gate that reads two chains' entries, an input, or a flag something else
/// derives, or more atoms than that, leaves every field the atoms reach to the
/// checks.
fn membership_groups(
    model: &CodecModel,
    roles: &CodecRoles,
    derived_whole: &BTreeSet<String>,
    held: &BTreeMap<(String, u64), bool>,
) -> Vec<GateGroup> {
    // Each presented gated field with the atoms its gate reads, and whether it
    // reads nothing else.
    type Atoms = Vec<(GateVar, bool)>;
    let gated: Vec<(&CodecField, Atoms, bool)> = model
        .fields
        .iter()
        .filter(|f| !derived_whole.contains(&f.id))
        .filter_map(|f| {
            let (atoms, whole) = gate_atoms(model, f.present_if.as_ref()?, held);
            Some((f, atoms, whole))
        })
        .collect();
    let mut groups = Vec::new();
    for chain in &model.fields {
        let of_chain =
            |var: &GateVar| matches!(var, GateVar::Member { chain: c, .. } if *c == chain.id);
        // The least set of atoms closed under the gates that read one, from
        // the chain's memberships, in the order the gates first name them.
        let mut atoms: Vec<GateVar> = Vec::new();
        for (_, read, _) in &gated {
            for (var, _) in read {
                if of_chain(var) && !atoms.contains(var) {
                    atoms.push(var.clone());
                }
            }
        }
        if atoms.is_empty() {
            continue;
        }
        loop {
            let reached: Vec<GateVar> = gated
                .iter()
                .filter(|(_, read, _)| read.iter().any(|(v, _)| atoms.contains(v)))
                .flat_map(|(_, read, _)| read.iter().map(|(v, _)| v.clone()))
                .filter(|v| !atoms.contains(v))
                .collect();
            if reached.is_empty() {
                break;
            }
            for var in reached {
                if !atoms.contains(&var) {
                    atoms.push(var);
                }
            }
        }
        let reached: Vec<&(&CodecField, Atoms, bool)> = gated
            .iter()
            .filter(|(_, read, _)| read.iter().any(|(v, _)| atoms.contains(v)))
            .collect();
        let decidable = atoms.len() <= MOST_GROUP_ATOMS
            && reached.iter().all(|(_, _, whole)| *whole)
            && atoms.iter().all(|var| match var {
                GateVar::Member { .. } => of_chain(var),
                GateVar::Flag { carrier, flag } => matches!(
                    roles.role_of(&ElementId::Flag {
                        field: carrier.clone(),
                        flag: flag.clone(),
                    }),
                    Some(Role::Presented)
                ),
                GateVar::Input(_) => false,
            });
        if !decidable {
            continue;
        }
        // The memberships first, then the flags, each in the order the gates
        // name them.
        let (mut gates, flags): (Vec<GateVar>, Vec<GateVar>) = atoms
            .into_iter()
            .partition(|var| matches!(var, GateVar::Member { .. }));
        gates.extend(flags);
        // A stated length is present where all of its content is absent, the
        // content being under one gate.
        let stated: Vec<(&str, &Atoms)> = model
            .fields
            .iter()
            .filter(|f| f.present_if.is_none())
            .filter(|f| crate::forge::codec_totality::stated(model, roles, f))
            .filter_map(|f| {
                let measured = crate::forge::codec_totality::measured(
                    model,
                    roles,
                    &ElementId::Field(f.id.clone()),
                );
                let gate_of = |id: &str| {
                    reached
                        .iter()
                        .find(|(g, _, _)| g.id == id)
                        .map(|(_, read, _)| read)
                };
                let first = gate_of(&measured.first()?.id)?;
                measured
                    .iter()
                    .all(|m| gate_of(&m.id) == Some(first))
                    .then_some((f.id.as_str(), first))
            })
            .collect();
        let k = gates.len();
        let mut assignments: Vec<Vec<bool>> = (0..1usize << k)
            .map(|mask| (0..k).map(|i| (mask >> (k - 1 - i)) & 1 == 1).collect())
            .collect();
        assignments.sort_by_key(|values: &Vec<bool>| {
            (
                std::cmp::Reverse(values.iter().filter(|v| **v).count()),
                values.iter().map(|v| !v).collect::<Vec<bool>>(),
            )
        });
        let arms = assignments
            .into_iter()
            .map(|values| {
                let fields = model
                    .fields
                    .iter()
                    .filter(|f| {
                        f.id == chain.id
                            || reached.iter().any(|(g, read, _)| {
                                g.id == f.id && gate_holds(read, &gates, &values)
                            })
                            || stated
                                .iter()
                                .any(|(id, read)| *id == f.id && !gate_holds(read, &gates, &values))
                    })
                    .map(|f| f.id.clone())
                    .collect();
                GroupArm { values, fields }
            })
            .collect();
        groups.push(GateGroup {
            gates,
            arms,
            absent: None,
            aligned: None,
        });
    }
    groups
}

/// The groups of the disjunctions of `model` (docs/adr/0065 §6 R4c): a gate
/// `a.X || b.Y` whose clauses each read a one-bit flag of the codec's own
/// carrier, a different one each, that no other gate reads and nothing else
/// derives. The fields under one such gate are present together, so they are
/// one group with an arm for each assignment of the flags that satisfies it,
/// and none under the one that does not. Any other disjunction stays a check.
fn disjunction_groups(
    model: &CodecModel,
    roles: &CodecRoles,
    derived_whole: &BTreeSet<String>,
    held: &BTreeMap<(String, u64), bool>,
) -> Vec<GateGroup> {
    // Each disjunction as the flags its clauses read and whether each is
    // negated, with the fields it gates.
    type Clauses = Vec<(GateVar, bool)>;
    let mut gates: Vec<(Clauses, Vec<String>)> = Vec::new();
    let mut flags_read: Vec<GateVar> = Vec::new();
    for field in &model.fields {
        let Some(gate) = &field.present_if else {
            continue;
        };
        let mut clauses: Vec<(GateVar, bool)> = Vec::new();
        let mut clause = Some(gate);
        let mut every_clause_a_flag = true;
        while let Some(p) = clause {
            let single = PresentIfPredicate {
                or_with: None,
                ..p.clone()
            };
            match gate_var(model, &single, held) {
                Some(var @ GateVar::Flag { .. }) => clauses.push((var, p.negate)),
                Some(var) => {
                    every_clause_a_flag = false;
                    flags_read.push(var);
                }
                None => every_clause_a_flag = false,
            }
            clause = p.or_with.as_deref();
        }
        flags_read.extend(clauses.iter().map(|(v, _)| v.clone()));
        if clauses.len() < 2 || !every_clause_a_flag || derived_whole.contains(&field.id) {
            continue;
        }
        match gates.iter_mut().find(|(c, _)| *c == clauses) {
            Some((_, fields)) => fields.push(field.id.clone()),
            None => gates.push((clauses, vec![field.id.clone()])),
        }
    }
    gates
        .into_iter()
        .filter(|(clauses, fields)| {
            let distinct: BTreeSet<&GateVar> = clauses.iter().map(|(v, _)| v).collect();
            distinct.len() == clauses.len()
                && clauses.iter().all(|(var, _)| {
                    // Read by the fields under this gate alone (once each), and
                    // derived by nothing else.
                    let GateVar::Flag { carrier, flag } = var else {
                        return false;
                    };
                    flags_read.iter().filter(|r| *r == var).count() == fields.len()
                        && matches!(
                            roles.role_of(&ElementId::Flag {
                                field: carrier.clone(),
                                flag: flag.clone(),
                            }),
                            Some(Role::Presented)
                        )
                })
        })
        .map(|(clauses, fields)| {
            let k = clauses.len();
            let assignment = |mask: usize| {
                (0..k)
                    .map(|i| (mask >> (k - 1 - i)) & 1 == 1)
                    .collect::<Vec<bool>>()
            };
            let holds = |values: &[bool]| {
                clauses
                    .iter()
                    .zip(values)
                    .any(|((_, negate), v)| v != negate)
            };
            // The arms by how many flags they hold set, then in the order the
            // gate names its flags.
            let mut assignments: Vec<Vec<bool>> = (0..1usize << k)
                .map(assignment)
                .filter(|values| holds(values))
                .collect();
            assignments.sort_by_key(|values| {
                (
                    values.iter().filter(|v| **v).count(),
                    values.iter().map(|v| !v).collect::<Vec<bool>>(),
                )
            });
            let arms = assignments
                .into_iter()
                .map(|values| GroupArm {
                    values,
                    fields: fields.clone(),
                })
                .collect();
            GateGroup {
                gates: clauses.iter().map(|(v, _)| v.clone()).collect(),
                arms,
                absent: Some(clauses.iter().map(|(_, negate)| *negate).collect()),
                aligned: None,
            }
        })
        .collect()
}

/// The flag a gate of one clause reads: a flag-input one bit wide, or a
/// one-bit flag of a carrier of the codec. Any other gate decides no group.
fn gate_var(
    model: &CodecModel,
    gate: &PresentIfPredicate,
    held: &BTreeMap<(String, u64), bool>,
) -> Option<GateVar> {
    if gate.or_with.is_some() {
        return None;
    }
    match gate.scope {
        PresentIfScope::Chain => gate.entry_id.and_then(|key| {
            held.get(&(gate.field_id.clone(), key))
                .map(|bare| GateVar::Member {
                    chain: gate.field_id.clone(),
                    key,
                    bare: *bare,
                })
        }),
        PresentIfScope::Input => model
            .flag_inputs
            .iter()
            .any(|i| i.name == gate.flag_name && i.width == 1)
            .then(|| GateVar::Input(gate.flag_name.clone())),
        PresentIfScope::Local => model
            .fields
            .iter()
            .find(|c| c.id == gate.field_id)
            .and_then(|c| c.flags.iter().find(|d| d.name == gate.flag_name))
            .filter(|d| d.width == 1)
            .map(|_| GateVar::Flag {
                carrier: gate.field_id.clone(),
                flag: gate.flag_name.clone(),
            }),
    }
}

/// A codec this one holds, with what its binds need.
struct HeldCodec {
    alias: String,
    /// Where this codec holds it: once for each field, and once for all the
    /// arms of the variant.
    sites: Vec<BindSite>,
    binds: Vec<FlagBind>,
    /// The inputs it says, among those it is bound or supplied.
    said: Vec<String>,
    /// Those of them it says in some of its values only.
    partly: Vec<String>,
}

impl HeldCodec {
    fn says(&self, input: &str) -> bool {
        self.said.iter().any(|s| s == input)
    }

    fn says_partly(&self, input: &str) -> bool {
        self.partly.iter().any(|s| s == input)
    }
}

/// The codecs `model` holds in a field or an arm, in declaration order.
fn held_codecs(model: &CodecModel, inputs: &mut dyn InputImports) -> Vec<HeldCodec> {
    let mut held: Vec<HeldCodec> = Vec::new();
    let mut add = |alias: &str, site: BindSite| match held.iter_mut().find(|h| h.alias == alias) {
        Some(h) => {
            if !(site == BindSite::Arm && h.sites.contains(&BindSite::Arm)) {
                h.sites.push(site);
            }
        }
        None => held.push(HeldCodec {
            alias: alias.to_string(),
            sites: vec![site],
            binds: Vec::new(),
            said: Vec::new(),
            partly: Vec::new(),
        }),
    };
    for field in &model.fields {
        let Some(alias) = child_alias(field) else {
            continue;
        };
        let site = if field.is_embed() {
            BindSite::Embed {
                field: field.id.clone(),
            }
        } else {
            BindSite::List {
                field: field.id.clone(),
            }
        };
        add(alias, site);
    }
    if let Some(variant) = &model.variant {
        for arm in variant.arms.iter().chain(variant.default_arm.iter()) {
            add(&arm.body_alias, BindSite::Arm);
        }
    }
    for h in &mut held {
        h.binds = inputs.flag_binds(&h.alias);
        let mut asked: Vec<String> = h.binds.iter().map(|b| b.input.clone()).collect();
        for field in &model.fields {
            if let BitSize::TlvChain { entry_binds, .. } = &field.bit_size {
                if child_alias(field) == Some(h.alias.as_str()) {
                    asked.extend(entry_binds.iter().map(|b| b.input.clone()));
                }
            }
        }
        h.said = asked
            .into_iter()
            .filter(|input| inputs.says_input(&h.alias, input))
            .collect();
        h.partly = h
            .said
            .iter()
            .filter(|input| inputs.says_input_partly(&h.alias, input))
            .cloned()
            .collect();
    }
    held
}

/// The flag-inputs of `model` its value says (docs/adr/0065 §6 R4a): those
/// exactly one thing says, a presented field gated on the input alone, the
/// group of the fields it decides (§6 R4b), the one body, always there, it is
/// handed on to, whose codec says it, the variant, every arm of which it is
/// handed on to and whose codec says it (§6 R4d), or the one list it is handed
/// on to, whose law holds it (§6 R8; `lawful` says which list can). A length or
/// a count gated with its content is derived whole and says nothing.
fn said_inputs(
    model: &CodecModel,
    held: &[HeldCodec],
    derived_whole: &BTreeSet<String>,
    groups: &[GateGroup],
    lawful: &dyn Fn(&str) -> bool,
) -> BTreeMap<String, InputSource> {
    let mut said = BTreeMap::new();
    for input in model.flag_inputs.iter().filter(|i| i.width == 1) {
        let mut gates: Vec<(&str, bool)> = Vec::new();
        let mut in_other_gate = false;
        for field in &model.fields {
            let Some(gate) = &field.present_if else {
                continue;
            };
            if !gate_names_input(gate, &input.name) {
                continue;
            }
            if gate.scope == PresentIfScope::Input && gate.or_with.is_none() {
                if !derived_whole.contains(&field.id) {
                    gates.push((&field.id, gate.negate));
                }
            } else {
                in_other_gate = true;
            }
        }
        let forwards: Vec<(&HeldCodec, &FlagBind)> = held
            .iter()
            .flat_map(|h| h.binds.iter().map(move |b| (h, b)))
            .filter(
                |(_, b)| matches!(&b.source, FlagBindSource::Input { name } if name == &input.name),
            )
            .collect();
        let group = groups
            .iter()
            .find(|g| g.gates == [GateVar::Input(input.name.clone())]);
        let source = match (gates.as_slice(), forwards.as_slice(), in_other_gate) {
            (gated, [], false)
                if group.is_some_and(|g| gated.iter().all(|(f, _)| g.contains(f))) =>
            {
                Some(InputSource::Group)
            }
            ([(field, when_clear)], [], false) => Some(InputSource::Gate {
                field: field.to_string(),
                when_clear: *when_clear,
            }),
            ([], forwards, false) if every_arm_says(model, forwards) => {
                Some(InputSource::EveryArm {
                    arms: bound_arms(forwards),
                })
            }
            ([], forwards, false) if some_arm_says(model, forwards) => {
                Some(InputSource::SomeArms {
                    arms: bound_arms(forwards),
                })
            }
            ([], [(h, bind)], false) if h.says(&bind.input) => match h.sites.as_slice() {
                [BindSite::Embed { field }]
                    if model
                        .fields
                        .iter()
                        .any(|f| &f.id == field && f.present_if.is_none()) =>
                {
                    Some(InputSource::Forward {
                        embed: field.clone(),
                        alias: h.alias.clone(),
                        input: bind.input.clone(),
                        partly: h.says_partly(&bind.input),
                    })
                }
                [BindSite::List { field }] if lawful(field) => Some(InputSource::List {
                    list: field.clone(),
                    alias: h.alias.clone(),
                    input: bind.input.clone(),
                    partly: model
                        .fields
                        .iter()
                        .any(|f| &f.id == field && f.present_if.is_some()),
                }),
                _ => None,
            },
            _ => None,
        };
        if let Some(source) = source {
            said.insert(input.name.clone(), source);
        }
    }
    said
}

/// Whether `forwards`, the binds that hand one input on, say it through the
/// variant of `model` alone: each is held in arms and nowhere else, and says the
/// one input it is bound.
fn arms_alone_say(model: &CodecModel, forwards: &[(&HeldCodec, &FlagBind)]) -> bool {
    let one_bind_each = forwards
        .iter()
        .map(|(h, _)| h.alias.as_str())
        .collect::<BTreeSet<_>>()
        .len()
        == forwards.len();
    model.variant.is_some()
        && !forwards.is_empty()
        && one_bind_each
        && forwards
            .iter()
            .all(|(h, bind)| h.sites == [BindSite::Arm] && h.says(&bind.input))
}

/// Whether every value of `model` says the input `forwards` hand on: every arm,
/// the default among them, is the codec of one of them and says it in all its
/// values. A value holds one arm, so one thing says the input in any value.
fn every_arm_says(model: &CodecModel, forwards: &[(&HeldCodec, &FlagBind)]) -> bool {
    let Some(variant) = &model.variant else {
        return false;
    };
    arms_alone_say(model, forwards)
        && forwards.iter().all(|(h, bind)| !h.says_partly(&bind.input))
        && variant
            .arms
            .iter()
            .chain(variant.default_arm.iter())
            .all(|arm| forwards.iter().any(|(h, _)| h.alias == arm.body_alias))
}

/// Whether the arms of `model` say the input `forwards` hand on and the others do
/// not read it. An arm that is not handed an input has no bit it could change, so
/// no value leaves a parent anything to hand.
fn some_arm_says(model: &CodecModel, forwards: &[(&HeldCodec, &FlagBind)]) -> bool {
    arms_alone_say(model, forwards)
}

/// The input each arm's codec is bound, by the alias it is imported as.
fn bound_arms(forwards: &[(&HeldCodec, &FlagBind)]) -> BTreeMap<String, String> {
    forwards
        .iter()
        .map(|(h, bind)| (h.alias.clone(), bind.input.clone()))
        .collect()
}

/// Whether a clause of `gate` reads the flag-input `name`.
fn gate_names_input(gate: &PresentIfPredicate, name: &str) -> bool {
    let mut clause = Some(gate);
    while let Some(p) = clause {
        if p.scope == PresentIfScope::Input && p.flag_name == name {
            return true;
        }
        clause = p.or_with.as_deref();
    }
    false
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
        inputs: BTreeMap::new(),
        input_checks: Vec::new(),
        list_laws: Vec::new(),
        groups: Vec::new(),
        unresolved: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Zenoh extension entry: a header whose identifier is the whole byte
    /// but its continuation flag `Z`, and a variant on the header's `enc` bits
    /// whose unit arm (0) has a body that presents nothing.
    fn ext_entry() -> EntryShape {
        let label = crate::DocumentLabel {
            identifier: "codec_zenoh_ext_entry",
            diagnostic_label: ".scxml-fixture",
        };
        let xml = include_str!("../../../tests/forge/resources/codec_zenoh_ext_entry.scxml");
        let model = match crate::forge::parser::parse_forge(xml, label) {
            Ok(Some(crate::forge::model::ForgeDocument::Codec(m))) => m,
            _ => panic!("the extension entry is a codec document"),
        };
        EntryShape {
            fields: model.fields,
            variant: model.variant,
            empty_arm_tags: vec![0],
        }
    }

    fn whole_header_but_z() -> TlvEntryId {
        TlvEntryId {
            carrier: None,
            name: "header".to_string(),
            except: vec!["Z".to_string()],
        }
    }

    #[test]
    fn an_entry_whose_identifier_selects_an_empty_arm_is_bare() {
        // 0x12: identifier 2, mandatory, the unit encoding.
        assert!(named_entry_is_bare(
            &ext_entry(),
            &whole_header_but_z(),
            Some("Z"),
            0x12
        ));
    }

    #[test]
    fn an_entry_whose_identifier_selects_an_arm_with_a_body_has_content() {
        // 0x32: the same identifier and the zint encoding, whose body is a number.
        assert!(!named_entry_is_bare(
            &ext_entry(),
            &whole_header_but_z(),
            Some("Z"),
            0x32
        ));
    }

    #[test]
    fn an_identifier_that_leaves_out_what_the_chain_does_not_write_has_content() {
        // A flag left out that is not the one ending the chain is the value's.
        assert!(!named_entry_is_bare(
            &ext_entry(),
            &whole_header_but_z(),
            None,
            0x12
        ));
        // An identifier that is one flag of the header leaves the rest to the entry.
        let ext_id = TlvEntryId {
            carrier: Some("header".to_string()),
            name: "ext_id".to_string(),
            except: Vec::new(),
        };
        assert!(!named_entry_is_bare(&ext_entry(), &ext_id, Some("Z"), 0x2));
    }
}
