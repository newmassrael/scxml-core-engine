// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//!   element that holds it;
//! - of a CBOR map, the tree over the type of each entry ([`CborShape`]) and
//!   every order its maps are written in (docs/adr/0101, section 5.1).

use crate::forge::bit_layout::{self, Piece, Slot};
use crate::forge::codec_roles::{
    self, BindSite, CodecRoles, Derivation, ElementId, GateGroup, InputValue, Role,
};
use crate::forge::codec_totality::{self, RefusalKind, Totality};
use crate::forge::generator::ImportContext;
use crate::forge::model::{
    BitSize, CborEntry, CodecEncoding, CodecField, CodecModel, ContentLineEntry, ContentLineModel,
    FlagBindSource, FlagDef, FlagInput, PresentIfPredicate, PresentIfScope, SceType,
    TlvTerminateStrategy,
};
use crate::forge::value_type::{TypeEnv, ValueType};
use std::collections::BTreeMap;

/// The encode plan of a codec, by the encoding it declares (docs/adr/0081 §1):
/// each encoding derives and writes by rules of its own, and a printer prints
/// the one the codec has.
pub enum CodecPlan<'m> {
    /// Fields at byte and bit positions, or streamed in turn. The largest plan,
    /// held apart so that the others are not its size.
    Positional(Box<EncodePlan<'m>>),
    /// One CBOR map.
    Cbor(CborPlan<'m>),
    /// One component of content lines.
    ContentLine(ContentLinePlan<'m>),
}

impl<'m> CodecPlan<'m> {
    /// The plan of the codec `model`, given what its imports are.
    pub fn of(model: &'m CodecModel, imports: &[ImportContext]) -> Self {
        match model.encoding {
            CodecEncoding::Positional => {
                CodecPlan::Positional(Box::new(EncodePlan::of(model, imports)))
            }
            CodecEncoding::Cbor => CodecPlan::Cbor(CborPlan::of(model, imports)),
            CodecEncoding::ContentLine => CodecPlan::ContentLine(ContentLinePlan::of(model)),
        }
    }

    /// Where the encode can refuse a value.
    pub fn totality(&self) -> &Totality {
        match self {
            CodecPlan::Positional(plan) => &plan.totality,
            CodecPlan::Cbor(plan) => &plan.totality,
            CodecPlan::ContentLine(plan) => &plan.totality,
        }
    }
}

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
    /// The flag inputs the encode is handed: those the value does not say
    /// (docs/adr/0065 §6 R4a), in declaration order.
    pub given: Vec<&'m FlagInput>,
    /// The flag inputs the value says, in declaration order.
    pub said: Vec<Said<'m>>,
    /// For each codec the value holds, by its import alias, what the codec is
    /// handed for each flag input it takes and does not say itself, in the
    /// order it declares them.
    pub handed: BTreeMap<String, Vec<Handed<'m>>>,
    /// Each input a held codec says that the parent also hands it from
    /// elsewhere, which the encode holds the codec to before it writes
    /// (docs/adr/0065 §6 R4a), in the order of the roles.
    pub input_checks: Vec<InputCheck<'m>>,
    /// Each gated field whose presence the encode cannot derive from its gate,
    /// which the encode holds to the gate before it writes, in the order of
    /// the roles.
    pub gate_checks: Vec<&'m CodecField>,
    /// How each chain that declares `entry-id` identifies its entries, by the
    /// chain's id.
    pub entry_ids: BTreeMap<String, EntryId>,
    /// The flags a parent may set in place of the ones the value presents: a
    /// one-bit flag the value presents of a fixed carrier it presents. A chain
    /// sets the flag its entries end on this way, as an entry cannot know where
    /// it stands.
    pub overridable: Vec<(&'m CodecField, &'m FlagDef)>,
    /// The writes of the encode, in wire order.
    pub program: Vec<Write<'m>>,
}

/// One write of the encode.
pub enum Write<'m> {
    /// A whole fixed field of a positional layout that shares none of its
    /// bytes, written as one value of its bytes in its byte order
    /// (docs/adr/0095 §2).
    Whole(&'m CodecField),
    /// Byte `at` of a positional layout: the or of the piece of each fixed
    /// field with bits in it, zero where no field has (docs/adr/0095 §2).
    Byte {
        /// The byte of the frame.
        at: u32,
        /// Each field with bits in the byte, and its piece, in declaration
        /// order.
        pieces: Vec<(&'m CodecField, Piece)>,
    },
    /// A variable-length field of a positional layout, after zeros up to its
    /// byte offset where the layout aligns its fields (`pad`).
    Tail {
        /// The field.
        field: &'m CodecField,
        /// Whether zeros fill the frame up to the field's offset first.
        pad: bool,
    },
    /// A field of a streamed layout, in turn.
    Streamed {
        /// The field.
        field: &'m CodecField,
        /// When it is written.
        guard: Guard,
        /// How a list or a chain is walked; `None` for any other field.
        list: Option<ListWrite>,
    },
    /// The body of the arm of the variant the value holds.
    Variant,
}

/// When a field of a streamed layout is written.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Guard {
    /// Always.
    Always,
    /// Where the value holds it: it is gated, or read from the arm of a group
    /// (a chain of memberships, docs/adr/0065 §6 R6a, or a list whose count
    /// another shares, R5), and presented.
    Present,
    /// Where its gate holds: it is gated and derived whole, as the gate flag
    /// the encode derived says.
    Gate,
}

/// How the encode walks a list or a chain as it writes it.
pub struct ListWrite {
    /// The inputs the entry codec is handed from the entry before: for each,
    /// the input and the kind of entry that sets it (0 for the first entry).
    pub previous: Vec<(String, u64)>,
    /// Where a group holds the chain's membership of a bare entry (docs/adr/0065
    /// §6 R6b), the kind of the named entry, which is written from the kind's
    /// key and whose identifier is that kind.
    pub slots: Option<u64>,
    /// Where the chain ends on its entries' flag, the entry codec's carrier and
    /// flag, set on every entry but the last.
    pub ends: Option<(String, String)>,
    /// Whether the list is held to its law and walked through it, which holds
    /// each entry to the law again (docs/adr/0065 §6 R8).
    pub lawful: bool,
}

/// An input a held codec says, held to what the parent hands it from
/// elsewhere: a value whose codec says otherwise has no frame, and the encode
/// refuses it with [`RefusalKind::PresentIfMismatch`] at [`InputCheck::at`].
pub struct InputCheck<'m> {
    /// The import alias of the held codec.
    pub codec: String,
    /// Its input.
    pub input: String,
    /// Whether the codec says the input in some of its values only, and a
    /// value that says nothing of it disagrees with nothing (§6 R4e).
    pub partly: bool,
    /// The element the refusal names: the field, or the alias of an arm's
    /// codec.
    pub at: String,
    /// Where the parent holds the codec.
    pub site: CheckSite<'m>,
    /// What the parent hands it.
    pub against: Against<'m>,
}

/// Where a parent holds a codec whose input it checks.
pub enum CheckSite<'m> {
    /// Every entry of the list `list`.
    List {
        /// The list.
        list: &'m CodecField,
    },
    /// The embedded body `body`.
    Embed {
        /// The body.
        body: &'m CodecField,
    },
    /// The arms of the variant whose codec it is, by their places among
    /// [`EncodePlan::arms`].
    Arm {
        /// The arms.
        arms: Vec<usize>,
    },
}

/// What a parent hands a held codec for an input it checks.
pub enum Against<'m> {
    /// The bit `bit` of the carrier `carrier`, as the encode writes it.
    Carrier {
        /// The carrier.
        carrier: &'m CodecField,
        /// The bit.
        bit: u32,
    },
    /// The parent's own flag input `name`.
    Input {
        /// The input.
        name: String,
    },
    /// Whether the entry before, in the chain [`CheckSite::List`] names, is of
    /// the kind `id` (false for the first entry).
    PreviousEntry {
        /// The kind.
        id: u64,
    },
}

/// How a chain identifies an entry, read off the entry's source as a `u64`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EntryId {
    /// The bits `mask` at `bit` of the entry's carrier `carrier`.
    Flag {
        /// The entry codec's carrier.
        carrier: String,
        /// The lowest bit.
        bit: u32,
        /// The bits, below the lowest.
        mask: u64,
    },
    /// The entry's field `field`, less the flags `mask` leaves out
    /// (`entry-id-except`).
    Whole {
        /// The entry codec's field.
        field: String,
        /// The bits kept, where some are left out.
        mask: Option<u64>,
    },
}

/// One clause of a gate, the clauses of a gate joined by "or".
pub enum Clause<'m> {
    /// The bit `bit` of the carrier `carrier` is set (clear where `negate`).
    Flag {
        /// The carrier.
        carrier: &'m CodecField,
        /// The bit.
        bit: u32,
        /// Whether the clause holds where the bit is clear.
        negate: bool,
    },
    /// The flag input `name` is set (clear where `negate`).
    Input {
        /// The input.
        name: String,
        /// Whether the clause holds where the input is clear.
        negate: bool,
    },
    /// The chain `chain` holds an entry of the kind `id` (none where `negate`).
    Member {
        /// The chain.
        chain: &'m CodecField,
        /// The kind.
        id: u64,
        /// Whether the clause holds where the chain holds none.
        negate: bool,
        /// Where a group holds the chain's memberships (docs/adr/0065 §6 R6),
        /// the group, by its place among [`EncodePlan::groups`], and the member
        /// of it this kind is, where it is one: the clause is then what the arm
        /// the value holds says.
        held: Option<(usize, Option<usize>)>,
    },
}

/// A flag input the value says (docs/adr/0065 §6 R4a).
pub struct Said<'m> {
    /// The input.
    pub input: &'m FlagInput,
    /// Whether a gate or a check of the codec reads it, so that the encode
    /// computes it before them.
    pub read: bool,
}

/// What a held codec is handed for one of its flag inputs.
pub struct Handed<'m> {
    /// The input of the held codec.
    pub input: String,
    /// What the parent hands it, or `None` where the parent binds nothing to it
    /// and a caller that writes the codec supplies it itself (a chain hands each
    /// entry what the entry before it was).
    pub from: Option<HandedFrom<'m>>,
}

/// What a parent hands a held codec for a flag input.
pub enum HandedFrom<'m> {
    /// The bits `width` wide at `bit` of the carrier `carrier`, as the encode
    /// writes it.
    Carrier {
        /// The carrier.
        carrier: &'m CodecField,
        /// The lowest bit.
        bit: u32,
        /// How many bits.
        width: u32,
    },
    /// The parent's own flag input `name`, passed on.
    Input {
        /// The parent's input.
        name: String,
    },
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
    /// Whether a value made by default holds this arm: the arm marked
    /// `default="true"`, else the first arm the variant names, else the
    /// default arm. Exactly one arm of a variant is it.
    pub by_default: bool,
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
    /// as `codec`. A body a gate leaves out has no frame to measure, so the
    /// measure is there only where the body is, as a gated field's length is
    /// (docs/adr/0058 §1).
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
        let given = model
            .flag_inputs
            .iter()
            .filter(|input| !roles.says_input(&input.name))
            .collect();
        let said = model
            .flag_inputs
            .iter()
            .filter(|input| roles.says_input(&input.name))
            .map(|input| Said {
                input,
                read: input_read(model, &roles, &input.name),
            })
            .collect();
        let handed = handed(model, imports);
        let input_checks = input_checks(model, &roles, &arms, imports);
        let gate_checks = roles
            .source_checks()
            .filter_map(|check| model.fields.iter().find(|f| f.id == check.field))
            .filter(|field| field.present_if.is_some())
            .collect();
        let entry_ids = entry_ids(model, imports);
        let overridable = overridable(model, &roles);
        let program = program(model, &roles, imports);
        EncodePlan {
            model,
            roles,
            totality,
            elements,
            arms,
            derivations,
            given,
            said,
            handed,
            input_checks,
            gate_checks,
            entry_ids,
            overridable,
            program,
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

    /// The clauses of the gate `gate`, a gate of this codec, in the order its
    /// author joined them.
    ///
    /// # Panics
    ///
    /// Where a clause names a carrier, a flag or a chain the codec does not
    /// hold, which the parser refuses.
    pub fn clauses(&self, gate: &PresentIfPredicate) -> Vec<Clause<'m>> {
        let field = |id: &str| self.model.fields.iter().find(|f| f.id == id);
        let mut out = Vec::new();
        let mut clause = Some(gate);
        while let Some(p) = clause {
            out.push(match p.scope {
                PresentIfScope::Chain => {
                    let chain =
                        field(&p.field_id).expect("the parser found the chain a gate reads");
                    let id = p.entry_id.expect("a chain clause names an identifier");
                    let held = self
                        .roles
                        .groups
                        .iter()
                        .position(|g| g.contains(&chain.id))
                        .filter(|&g| self.roles.groups[g].chain() == Some(chain.id.as_str()))
                        .map(|g| {
                            let member = self.roles.groups[g]
                                .members()
                                .find(|(_, key)| *key == id)
                                .map(|(index, _)| index);
                            (g, member)
                        });
                    Clause::Member {
                        chain,
                        id,
                        negate: p.negate,
                        held,
                    }
                }
                PresentIfScope::Input => Clause::Input {
                    name: p.flag_name.clone(),
                    negate: p.negate,
                },
                _ => {
                    let carrier =
                        field(&p.field_id).expect("the parser found the carrier a gate reads");
                    let def = carrier
                        .flags
                        .iter()
                        .find(|d| d.name == p.flag_name)
                        .expect("the parser found the flag a gate reads");
                    Clause::Flag {
                        carrier,
                        bit: def.bit,
                        negate: p.negate,
                    }
                }
            });
            clause = p.or_with.as_deref();
        }
        out
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
                                gated: body.present_if.is_some(),
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

/// Whether a gate of a field the value does not present whole, or a check of
/// an input a held codec says, reads the flag input `name`.
fn input_read(model: &CodecModel, roles: &CodecRoles, name: &str) -> bool {
    let gated = model.fields.iter().any(|f| {
        !codec_totality::presented(roles, f)
            && f.present_if.as_ref().is_some_and(|gate| {
                let mut clause = Some(gate);
                while let Some(p) = clause {
                    if p.scope == PresentIfScope::Input && p.flag_name == name {
                        return true;
                    }
                    clause = p.or_with.as_deref();
                }
                false
            })
    });
    gated
        || roles
            .input_checks
            .iter()
            .any(|c| matches!(&c.against, InputValue::Input { name: n } if n == name))
}

/// What each codec `model` imports is handed for each flag input it takes and
/// does not say itself.
fn handed<'m>(
    model: &'m CodecModel,
    imports: &[ImportContext],
) -> BTreeMap<String, Vec<Handed<'m>>> {
    imports
        .iter()
        .map(|import| {
            let handed = import
                .codec_flag_inputs
                .iter()
                // An input the codec says is not handed to it.
                .filter(|input| !import.codec_says_inputs.iter().any(|s| s == &input.name))
                .map(|input| Handed {
                    input: input.name.clone(),
                    from: import
                        .flag_binds
                        .iter()
                        .find(|b| b.input == input.name)
                        .and_then(|bind| match &bind.source {
                            FlagBindSource::Carrier { carrier, flag } => {
                                let carrier = model
                                    .fields
                                    .iter()
                                    .find(|f| f.is_flags_carrier() && &f.id == carrier)?;
                                let def = carrier.flags.iter().find(|d| &d.name == flag)?;
                                Some(HandedFrom::Carrier {
                                    carrier,
                                    bit: def.bit,
                                    width: def.width.max(1),
                                })
                            }
                            FlagBindSource::Input { name } => {
                                Some(HandedFrom::Input { name: name.clone() })
                            }
                        }),
                })
                .collect();
            (import.alias.clone(), handed)
        })
        .collect()
}

/// Each input a held codec says that the parent also hands it from elsewhere
/// ([`codec_roles::InputCheck`]).
fn input_checks<'m>(
    model: &'m CodecModel,
    roles: &CodecRoles,
    arms: &[Arm<'m>],
    imports: &[ImportContext],
) -> Vec<InputCheck<'m>> {
    let field = |id: &str| {
        model
            .fields
            .iter()
            .find(|f| f.id == id)
            .expect("a bind site is a field of the codec")
    };
    roles
        .input_checks
        .iter()
        .map(|check| {
            let binder = &check.binder;
            let site = match &binder.site {
                BindSite::List { field: id } => CheckSite::List { list: field(id) },
                BindSite::Embed { field: id } => CheckSite::Embed { body: field(id) },
                BindSite::Arm => CheckSite::Arm {
                    arms: arms
                        .iter()
                        .enumerate()
                        .filter(|(_, arm)| arm.codec == binder.alias)
                        .map(|(index, _)| index)
                        .collect(),
                },
            };
            let against = match &check.against {
                InputValue::Carrier { carrier, flag } => {
                    let carrier = field(carrier);
                    let def = carrier
                        .flags
                        .iter()
                        .find(|d| &d.name == flag)
                        .expect("the parser found the flag a bind names");
                    Against::Carrier {
                        carrier,
                        bit: def.bit,
                    }
                }
                InputValue::Input { name } => Against::Input { name: name.clone() },
                InputValue::PreviousEntry { id } => Against::PreviousEntry { id: *id },
            };
            InputCheck {
                codec: binder.alias.clone(),
                input: binder.input.clone(),
                partly: imports.iter().any(|i| {
                    i.alias == binder.alias
                        && i.codec_partly_says_inputs
                            .iter()
                            .any(|s| s == &binder.input)
                }),
                at: binder.site.element(&binder.alias).to_string(),
                site,
                against,
            }
        })
        .collect()
}

/// How each chain of `model` that declares `entry-id` identifies its entries.
fn entry_ids(model: &CodecModel, imports: &[ImportContext]) -> BTreeMap<String, EntryId> {
    let masks = crate::forge::generator::chain_identifier_masks(model, imports);
    model
        .fields
        .iter()
        .filter_map(|chain| {
            let BitSize::TlvChain {
                entry_id: Some(identifier),
                ..
            } = &chain.bit_size
            else {
                return None;
            };
            let alias = codec_totality::child_alias(chain)?;
            let import = imports.iter().find(|i| i.alias == alias)?;
            let id = match &identifier.carrier {
                Some(carrier_id) => {
                    let carrier = import.codec_fields.iter().find(|f| &f.id == carrier_id)?;
                    let def = carrier.flags.iter().find(|d| d.name == identifier.name)?;
                    EntryId::Flag {
                        carrier: carrier.id.clone(),
                        bit: def.bit,
                        mask: (1u64 << def.width) - 1,
                    }
                }
                None => EntryId::Whole {
                    field: identifier.name.clone(),
                    mask: masks.get(&chain.id).copied(),
                },
            };
            Some((chain.id.clone(), id))
        })
        .collect()
}

/// The one-bit flags the value presents of the fixed carriers it presents.
fn overridable<'m>(
    model: &'m CodecModel,
    roles: &CodecRoles,
) -> Vec<(&'m CodecField, &'m FlagDef)> {
    let mut out = Vec::new();
    for carrier in &model.fields {
        if carrier.fixed_bits().is_none() || !codec_totality::presented(roles, carrier) {
            continue;
        }
        for def in &carrier.flags {
            let id = ElementId::Flag {
                field: carrier.id.clone(),
                flag: def.name.clone(),
            };
            if def.width == 1 && matches!(roles.role_of(&id), Some(Role::Presented)) {
                out.push((carrier, def));
            }
        }
    }
    out
}

/// The writes of the encode of `model`, in wire order: a positional layout's
/// fixed bytes (docs/adr/0095 §2), then its variable-length fields; or a
/// streamed layout's fields in turn; then the variant's arm.
fn program<'m>(
    model: &'m CodecModel,
    roles: &CodecRoles,
    imports: &[ImportContext],
) -> Vec<Write<'m>> {
    let mut out = Vec::new();
    if model.positional_layout_is_valid() {
        for slot in bit_layout::slots(&model.fields, model.default_endian) {
            out.push(match slot {
                Slot::Whole(field) => Write::Whole(field),
                Slot::Byte { at, pieces } => Write::Byte { at, pieces },
            });
        }
        for field in model.fields.iter().filter(|f| f.is_variable_length()) {
            out.push(Write::Tail {
                field,
                pad: model.has_dma_aligned_fields(),
            });
        }
    } else {
        for field in &model.fields {
            out.push(Write::Streamed {
                field,
                guard: guard(roles, field),
                list: (field.is_repeat() || field.is_tlv_chain())
                    .then(|| list_write(roles, field, imports)),
            });
        }
    }
    if model.variant.is_some() {
        out.push(Write::Variant);
    }
    out
}

/// When the field `field` of a streamed layout is written.
fn guard(roles: &CodecRoles, field: &CodecField) -> Guard {
    let group = roles.group_of(&field.id);
    // The chain of the memberships is read from the arm the value holds, as an
    // `Option` of its walk whether or not it is itself gated (docs/adr/0065 §6
    // R6a), and so is the list a count is shared with, which every arm holds (R5).
    let membership = group.is_some_and(|g| g.chain() == Some(field.id.as_str()));
    let aligned = group.is_some_and(|g| g.aligned.as_ref().is_some_and(|(f, _)| *f == field.id));
    let gated = field.present_if.is_some() || membership || aligned;
    match (gated, codec_totality::presented(roles, field)) {
        (true, true) => Guard::Present,
        (true, false) => Guard::Gate,
        (false, _) => Guard::Always,
    }
}

/// How the encode walks the list or chain `field` as it writes it.
fn list_write(roles: &CodecRoles, field: &CodecField, imports: &[ImportContext]) -> ListWrite {
    let import = codec_totality::child_alias(field)
        .and_then(|alias| imports.iter().find(|i| i.alias == alias));
    let says = |input: &str| import.is_some_and(|i| i.codec_says_inputs.iter().any(|s| s == input));
    let (previous, ends) = match &field.bit_size {
        BitSize::TlvChain {
            entry_binds,
            terminate_on,
            ..
        } => (
            // An input the entry says is checked in the prologue and not handed
            // to it (docs/adr/0065 §6 R4a).
            entry_binds
                .iter()
                .filter(|bind| !says(&bind.input))
                .map(|bind| (bind.input.clone(), bind.previous_entry_id))
                .collect(),
            match terminate_on {
                TlvTerminateStrategy::EntryFlag { flag_name } => import
                    .and_then(|i| {
                        i.codec_fields
                            .iter()
                            .find(|f| f.flags.iter().any(|d| &d.name == flag_name))
                    })
                    .map(|carrier| (carrier.id.clone(), flag_name.clone())),
                _ => None,
            },
        ),
        _ => (Vec::new(), None),
    };
    let slots = roles
        .group_of(&field.id)
        .filter(|g| g.chain() == Some(field.id.as_str()) && g.member_is_bare())
        .map(|g| {
            g.members()
                .next()
                .expect("the one gate on a bare entry is a membership")
                .1
        });
    ListWrite {
        previous,
        slots,
        ends,
        lawful: !list_law(roles, &field.id).is_none(),
    }
}

/// The arms of the variant of `model`: those it names, in order, then the
/// default. A value made by default holds the arm marked `default="true"`,
/// else the first arm named, else the default arm.
fn arms(model: &CodecModel) -> Vec<Arm<'_>> {
    let Some(variant) = &model.variant else {
        return Vec::new();
    };
    let marked = variant.arms.iter().position(|arm| arm.is_default);
    let chosen = marked.or((!variant.arms.is_empty()).then_some(0));
    let mut arms: Vec<Arm<'_>> = variant
        .arms
        .iter()
        .enumerate()
        .map(|(at, arm)| Arm {
            codec: &arm.body_alias,
            tag: Some(arm.value),
            feature: arm.feature.as_deref(),
            by_default: chosen == Some(at),
        })
        .collect();
    if let Some(default) = &variant.default_arm {
        arms.push(Arm {
            codec: &default.body_alias,
            tag: None,
            feature: None,
            by_default: chosen.is_none(),
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

/// The encode plan of a CBOR codec (SCE_FORGE.md §4.6.1, docs/adr/0081 §1):
/// a tree over the type of each entry (docs/adr/0101, section 5.1).
///
/// A CBOR map derives nothing a value says: the number of entries in the map
/// head is what the encoding counts. Every entry is presented, required or
/// not, and the entries are written in ascending key order with the shortest
/// heads (RFC 8949 §4.2.1). What an entry holds is the shape of its type
/// ([`CborShape`]), each enum, record and union it names read in the codec's
/// imports, so every order the encode writes in is decided here, once: the
/// entries' ([`CborPlan::by_key`]) and each record's fields'
/// ([`CborRecord::by_key`]). A printer walks the plan and sorts nothing.
pub struct CborPlan<'m> {
    /// The codec the plan is of.
    pub model: &'m CodecModel,
    /// Where the encode can refuse a value, read from the shapes of the
    /// entries ([`codec_totality`]).
    pub totality: Totality,
    /// Each entry, in declaration order.
    pub entries: Vec<CborItem<'m>>,
    /// The entries' places among [`CborPlan::entries`] in the order the encode
    /// writes them: ascending key ([`entry_key_order`]).
    pub by_key: Vec<usize>,
}

/// One entry of a CBOR map.
pub struct CborItem<'m> {
    /// The entry.
    pub entry: &'m CborEntry,
    /// Whether every value holds it; the map head counts the others only where
    /// they are there, and an absent one is written by leaving its key out
    /// (docs/adr/0101, section 2.2).
    pub required: bool,
    /// What a present value of it is: the shape of its type outside an
    /// `optional<…>`.
    pub value: CborShape,
}

/// What a value of a CBOR codec's entry is, as the encode writes it
/// (docs/adr/0101, sections 2.1 and 5.1): the shape of its type, each enum,
/// record and union it names resolved. A printer that does not lower a shape
/// yet refuses the codec by name before it prints it
/// ([`crate::forge::cbor_codec::refusal`], docs/adr/0101 section 5.4).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CborShape {
    /// An integer of `bits` bits: major type 0, and major type 1 below zero.
    Int {
        /// 8, 16, 32 or 64.
        bits: u32,
        /// Whether it holds numbers below zero.
        signed: bool,
    },
    /// A float, written in the shortest of half, single and double precision
    /// that holds it exactly.
    Float {
        /// 32 or 64.
        bits: u32,
    },
    /// A boolean.
    Bool,
    /// A text of at most `max` UTF-8 bytes (docs/adr/0077 §3).
    Text {
        /// The bound its type states.
        max: u32,
    },
    /// A byte string of at most `max` bytes; of exactly `max` where `exact`,
    /// which only a codec's entry states, by `sce:length` (docs/adr/0101,
    /// section 1.2).
    Bytes {
        /// The bound its type states.
        max: u32,
        /// Whether its length is `max` and no other.
        exact: bool,
    },
    /// A value of an enum, written as the integer of its declared number.
    Enum(CborEnum),
    /// A record, written as a map of its present fields, each keyed by its id.
    Record(CborRecord),
    /// A union, written as a map of one entry keyed by its case's variant name.
    Union(CborUnion),
    /// A list of `min` to `max` elements, written as a definite-length array
    /// of its elements in their order.
    List {
        /// What each element is.
        elem: Box<CborShape>,
        /// The most elements it holds.
        max: u32,
        /// Its lower count (docs/adr/0086, section 15); 0 where it states none.
        min: u32,
    },
    /// A list's element that may be absent, written as `null` where it is
    /// (docs/adr/0101, section 2.1). An entry or a record's field that may be
    /// absent is written by leaving its key out, which its item or field says
    /// ([`CborItem::required`], [`CborField::optional`]): the grammar holds an
    /// `optional<…>` nowhere else.
    Nullable(Box<CborShape>),
    /// A part of the type the plan cannot say: an enum, a record or a union
    /// the codec's imports do not resolve, a record or a union that holds
    /// itself, or a text or a byte string with no bound. The compile refuses
    /// each before any printer reads the plan (the import check,
    /// [`crate::forge::cbor_codec::record_list_unlowered`], docs/adr/0101
    /// section 1.3); the plan names what it met.
    Unresolved {
        /// The alias, or the type, as the document spells it.
        what: String,
    },
}

impl CborShape {
    /// The shape of a value of `ty`, each enum, record and union it names read
    /// in `env`. An `optional<T>` read here is a list's element
    /// ([`CborShape::Nullable`]); an entry or a field that may be absent is
    /// asked of its element.
    pub fn of(ty: &ValueType, env: &dyn TypeEnv) -> Self {
        shape_of(ty, env, &mut Vec::new())
    }

    /// The record each element of this list is, the shape docs/adr/0085
    /// Landing order step 3a lowers; `None` for any other shape.
    pub fn listed_record(&self) -> Option<&CborRecord> {
        match self {
            CborShape::List { elem, .. } => match &**elem {
                CborShape::Record(record) => Some(record),
                _ => None,
            },
            _ => None,
        }
    }
}

/// The enum a CBOR value is of.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CborEnum {
    /// The alias that names the enum where it stands.
    pub alias: String,
    /// The integer type its numbers are (`sce:underlying-type`).
    pub carrier: SceType,
    /// Whether a number no variant declares is refused
    /// (`sce:strict-variants`).
    pub closed: bool,
}

/// A record a CBOR value holds: a map of its present fields, each keyed by its
/// id as a text string (docs/adr/0101, section 2.1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CborRecord {
    /// The alias that names the record where it stands.
    pub alias: String,
    /// The name of its event schema's document (its `<scxml name>`), which
    /// tells one record from another whatever alias names it.
    pub name: String,
    /// Its fields, in the schema's order.
    pub fields: Vec<CborField>,
    /// The fields' places among [`CborRecord::fields`] in the order the map
    /// writes them ([`text_key_order`]).
    pub by_key: Vec<usize>,
}

impl CborRecord {
    /// Its fields in the order its map writes them.
    pub fn fields_by_key(&self) -> impl Iterator<Item = &CborField> + '_ {
        self.by_key.iter().map(|&at| &self.fields[at])
    }
}

/// A field of a record a CBOR value holds.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CborField {
    /// The field's id, the text its key is.
    pub id: String,
    /// Its type, as the schema declares it.
    pub ty: ValueType,
    /// Whether it may be absent: an `optional<…>` field, whose key is left
    /// out where it is.
    pub optional: bool,
    /// What a present value of it is.
    pub shape: CborShape,
}

/// A union a CBOR value holds: a map of one entry, keyed by its case's variant
/// name as a text string, whose value is the case's record or the empty map
/// (docs/adr/0101, section 2.1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CborUnion {
    /// The alias that names the union where it stands.
    pub alias: String,
    /// The name of its document.
    pub name: String,
    /// Its cases, in the declaration order of its tag's enum.
    pub cases: Vec<CborCase>,
}

/// A case of a union a CBOR value holds.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CborCase {
    /// The name of the tag's variant, the text its key is.
    pub variant: String,
    /// The number the variant declares, which the generated type's tag holds.
    pub number: i128,
    /// The record it holds, or `None` for a case that holds none, written as
    /// the empty map.
    pub record: Option<CborRecord>,
}

/// The order a map whose keys are the texts `ids` writes them in: by the bytes
/// of each key's encoding (RFC 8949 §4.2.1). A text's head grows with its
/// length, so a shorter id comes first, and ids of one length are in the order
/// of their bytes. The one rule a record's map is written by.
pub fn text_key_order<'a>(ids: impl IntoIterator<Item = &'a str>) -> Vec<usize> {
    let ids: Vec<&'a str> = ids.into_iter().collect();
    let mut order: Vec<usize> = (0..ids.len()).collect();
    order.sort_by_key(|&at| (ids[at].len(), ids[at].as_bytes()));
    order
}

/// The order the codec's own map writes `entries` in: ascending key. Each key
/// is an unsigned integer below 24, whose encoding is the one byte of its head,
/// so this is the order of their encodings (docs/adr/0101, section 2.2).
pub fn entry_key_order(entries: &[CborEntry]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by_key(|&at| entries[at].key);
    order
}

/// Each entry of the CBOR codec `model`, in declaration order, the shape of
/// its present type read in `env`: what [`CborPlan::entries`] holds, and what
/// the totality analysis reads ([`codec_totality`]).
pub fn cbor_items<'m>(model: &'m CodecModel, env: &dyn TypeEnv) -> Vec<CborItem<'m>> {
    model
        .cbor_entries
        .iter()
        .map(|entry| {
            let mut value = CborShape::of(entry.present_type(), env);
            // An exact length is the entry's own (docs/adr/0101, section 1.2).
            if let (CborShape::Bytes { exact, .. }, Some(_)) = (&mut value, entry.length) {
                *exact = true;
            }
            CborItem {
                entry,
                required: entry.is_required(),
                value,
            }
        })
        .collect()
}

/// [`CborShape::of`], `records` holding each record and union on the way by its
/// key ([`TypeEnv::record_key`]), so that one that holds itself is met once.
fn shape_of(ty: &ValueType, env: &dyn TypeEnv, records: &mut Vec<String>) -> CborShape {
    match ty {
        ValueType::Scalar(t) => scalar_shape(t, env),
        ValueType::Text { max } => CborShape::Text { max: max.max },
        ValueType::Bytes { max } => CborShape::Bytes {
            max: max.max,
            exact: false,
        },
        ValueType::Optional { elem } => CborShape::Nullable(Box::new(shape_of(elem, env, records))),
        ValueType::List { elem, max } => CborShape::List {
            elem: Box::new(shape_of(elem, env, records)),
            max: max.max,
            min: max.min,
        },
        ValueType::Record { alias } => record_shape(alias, env, records)
            .map_or_else(|what| CborShape::Unresolved { what }, CborShape::Record),
        ValueType::Union { alias } => union_shape(alias, env, records)
            .map_or_else(|what| CborShape::Unresolved { what }, CborShape::Union),
    }
}

/// The shape of a scalar of the type grammar.
fn scalar_shape(t: &SceType, env: &dyn TypeEnv) -> CborShape {
    match t {
        SceType::Bool => CborShape::Bool,
        SceType::Float32 => CborShape::Float { bits: 32 },
        SceType::Float64 => CborShape::Float { bits: 64 },
        SceType::Enum(reference) => match env.enum_type(&reference.alias) {
            Some(enumeration) => CborShape::Enum(CborEnum {
                alias: reference.alias.clone(),
                carrier: enumeration.underlying.clone(),
                closed: enumeration.strict,
            }),
            None => CborShape::Unresolved {
                what: reference.alias.clone(),
            },
        },
        // The grammar reads a text and a byte string with their bound
        // (`ValueType::Text`, `ValueType::Bytes`); a scalar one has none.
        SceType::String => CborShape::Unresolved {
            what: "string".to_string(),
        },
        SceType::Bytes => CborShape::Unresolved {
            what: "bytes".to_string(),
        },
        SceType::Uint8
        | SceType::Uint16
        | SceType::Uint32
        | SceType::Uint64
        | SceType::Int8
        | SceType::Int16
        | SceType::Int32
        | SceType::Int64 => CborShape::Int {
            bits: t.int_bit_width().unwrap_or(64),
            signed: t.is_signed(),
        },
    }
}

/// The record `alias` names in `env`, its fields read in its own document's
/// scope; the alias where it does not resolve or holds itself.
fn record_shape(
    alias: &str,
    env: &dyn TypeEnv,
    records: &mut Vec<String>,
) -> Result<CborRecord, String> {
    let (fields, inner) = crate::forge::value_type::enter_record(alias, env, records)
        .map_err(|_| alias.to_string())?;
    let fields: Vec<CborField> = fields
        .into_iter()
        .map(|(id, ty)| {
            let present = ty.optional_elem().unwrap_or(&ty);
            let shape = shape_of(present, inner, records);
            CborField {
                optional: ty.optional_elem().is_some(),
                shape,
                id,
                ty,
            }
        })
        .collect();
    records.pop();
    Ok(CborRecord {
        alias: alias.to_string(),
        name: env.record_name(alias),
        by_key: text_key_order(fields.iter().map(|f| f.id.as_str())),
        fields,
    })
}

/// The union `alias` names in `env`, each case's record read in the union's
/// own document's scope; the alias of what does not resolve or holds itself.
fn union_shape(
    alias: &str,
    env: &dyn TypeEnv,
    records: &mut Vec<String>,
) -> Result<CborUnion, String> {
    let union = crate::forge::value_type::enter_union(alias, env, records)
        .map_err(|_| alias.to_string())?;
    let mut cases = Vec::with_capacity(union.cases.len());
    let mut unresolved = None;
    for case in &union.cases {
        let record = match &case.record {
            None => None,
            Some(record) => match record_shape(record, &union.scope, records) {
                Ok(record) => Some(record),
                Err(what) => {
                    unresolved = Some(what);
                    break;
                }
            },
        };
        cases.push(CborCase {
            variant: case.variant.name.clone(),
            number: case.variant.value,
            record,
        });
    }
    records.pop();
    match unresolved {
        Some(what) => Err(what),
        None => Ok(CborUnion {
            alias: alias.to_string(),
            name: union.name.clone(),
            cases,
        }),
    }
}

impl<'m> CborPlan<'m> {
    /// The plan of the CBOR codec `model`, given what its imports are: each
    /// entry's shape read in the types they name
    /// ([`crate::forge::generator::import_type_scope`]).
    pub fn of(model: &'m CodecModel, imports: &[ImportContext]) -> Self {
        let scope = crate::forge::generator::import_type_scope(imports);
        let entries = cbor_items(model, &scope);
        CborPlan {
            model,
            totality: codec_totality::cbor_totality(&entries),
            by_key: entry_key_order(&model.cbor_entries),
            entries,
        }
    }

    /// How many entries every value holds.
    pub fn required(&self) -> usize {
        self.entries.iter().filter(|item| item.required).count()
    }
}

/// The encode plan of a content-line codec (SCE_FORGE.md §4.6.4, docs/adr/0014,
/// docs/adr/0081 §1).
///
/// A component of content lines derives nothing a value says: every property
/// and parameter is presented. What the plan holds is the shape of each
/// property's lines, what each line holds, and which refusals precede a line:
/// a required property absent, more lines than the protocol states, a parameter
/// with no value of its property to stand on. Folding at 75 octets, escapes and
/// quoting are the runtime's line writer's.
pub struct ContentLinePlan<'m> {
    /// The codec the plan is of.
    pub model: &'m CodecModel,
    /// Its component and entries.
    pub lines: &'m ContentLineModel,
    /// Where the encode can refuse a value.
    pub totality: Totality,
    /// Each property, in declaration order: the order its lines are written in.
    pub properties: Vec<Property<'m>>,
}

/// One property of a content-line component.
pub struct Property<'m> {
    /// The property's entry.
    pub entry: &'m ContentLineEntry,
    /// The shape of its lines.
    pub shape: LineShape,
    /// Its parameters, in declaration order, written after its name.
    pub params: Vec<Param<'m>>,
    /// What one of its lines holds.
    pub value: LineValue,
}

/// The shape of a property's lines.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LineShape {
    /// Repeated, each line a record of its parameters and its value or values.
    Records {
        /// Whether a value holds one line or more; one with none is refused
        /// with [`RefusalKind::LineRequiredMissing`].
        required: bool,
        /// The most lines, where the protocol states it; more are refused with
        /// [`RefusalKind::LineTooMany`].
        at_most: Option<u32>,
    },
    /// Repeated, each line one value and no parameter.
    Repeated {
        /// As [`LineShape::Records::required`].
        required: bool,
        /// As [`LineShape::Records::at_most`].
        at_most: Option<u32>,
    },
    /// One line, or none where it is not required. A parameter with no value of
    /// its property to stand on is refused with
    /// [`RefusalKind::LineRequiredMissing`].
    One {
        /// Whether every value holds it.
        required: bool,
        /// Whether its line holds a list of values, absent where the list is
        /// empty.
        values: bool,
    },
}

/// A parameter of a property.
pub struct Param<'m> {
    /// The parameter's entry.
    pub entry: &'m ContentLineEntry,
    /// Whether every line holds it.
    pub required: bool,
    /// What it holds. The line writer can refuse every parameter's value, with
    /// [`RefusalKind::LineBadValue`].
    pub value: ParamValue,
}

/// What a parameter holds.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ParamValue {
    /// One of an enum's texts.
    Enum {
        /// The import alias of the enum.
        alias: String,
    },
    /// Text, of `max_size` bytes at most where the protocol states it.
    Text {
        /// The most bytes.
        max_size: Option<u32>,
    },
}

/// What one line of a property holds.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LineValue {
    /// Values joined by `separator`, of `max_values` at most and `max_size`
    /// bytes each at most where the protocol states them; escaped as TEXT where
    /// `text`. Refused by the line writer with [`RefusalKind::LineBadValue`].
    Values {
        /// The separator.
        separator: String,
        /// Whether the values are TEXT.
        text: bool,
        /// The most bytes of one value.
        max_size: Option<u32>,
        /// The most values.
        max_values: Option<u32>,
    },
    /// Text, of `max_size` bytes at most; escaped as TEXT where `text`. Refused
    /// by the line writer with [`RefusalKind::LineBadValue`].
    Text {
        /// Whether it is TEXT.
        text: bool,
        /// The most bytes.
        max_size: Option<u32>,
    },
    /// One of an enum's texts. Refused by the line writer with
    /// [`RefusalKind::LineBadValue`].
    Enum {
        /// The import alias of the enum.
        alias: String,
    },
    /// A boolean.
    Bool,
    /// An unsigned integer; `wide` where it is 64 bits.
    Uint {
        /// Whether it is 64 bits wide.
        wide: bool,
    },
    /// A signed integer; `wide` where it is 64 bits.
    Int {
        /// Whether it is 64 bits wide.
        wide: bool,
    },
}

impl<'m> ContentLinePlan<'m> {
    /// The plan of the content-line codec `model`.
    ///
    /// # Panics
    ///
    /// Where `model` carries no content-line entries.
    pub fn of(model: &'m CodecModel) -> Self {
        let lines = model
            .content_line
            .as_ref()
            .expect("a content-line codec carries its entries");
        let properties = lines
            .entries
            .iter()
            .filter(|entry| entry.param.is_none())
            .map(|entry| Property {
                entry,
                shape: if lines.reads_records(entry) {
                    LineShape::Records {
                        required: entry.is_required(),
                        at_most: entry.max_count,
                    }
                } else if entry.repeated {
                    LineShape::Repeated {
                        required: entry.is_required(),
                        at_most: entry.max_count,
                    }
                } else {
                    LineShape::One {
                        required: entry.is_required(),
                        values: lines.reads_values(entry),
                    }
                },
                params: lines
                    .parameters_of(entry)
                    .map(|param| Param {
                        entry: param,
                        required: param.is_required(),
                        value: match param.present_type() {
                            SceType::Enum(r) => ParamValue::Enum {
                                alias: r.alias.clone(),
                            },
                            _ => ParamValue::Text {
                                max_size: param.max_size,
                            },
                        },
                    })
                    .collect(),
                value: line_value(entry),
            })
            .collect();
        ContentLinePlan {
            model,
            lines,
            // A content-line codec imports no codec: its refusals are its own.
            totality: codec_totality::analyse(model, &mut codec_totality::Resolved(&[])),
            properties,
        }
    }
}

/// What one line of the property `entry` holds.
fn line_value(entry: &ContentLineEntry) -> LineValue {
    if let Some(separator) = &entry.separator {
        return LineValue::Values {
            separator: separator.clone(),
            text: entry.text,
            max_size: entry.max_size,
            max_values: entry.max_values,
        };
    }
    match entry.present_type() {
        SceType::String => LineValue::Text {
            text: entry.text,
            max_size: entry.max_size,
        },
        SceType::Enum(r) => LineValue::Enum {
            alias: r.alias.clone(),
        },
        SceType::Bool => LineValue::Bool,
        SceType::Uint64 => LineValue::Uint { wide: true },
        SceType::Int64 => LineValue::Int { wide: true },
        ty if ty.is_signed() => LineValue::Int { wide: false },
        _ => LineValue::Uint { wide: false },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::value_type::{
        ScopedCase, ScopedEnum, ScopedRecord, ScopedUnion, ScopedVariant, TypeScope, WrittenType,
    };

    fn ty(text: &str) -> ValueType {
        WrittenType::read(text)
            .and_then(WrittenType::bounded)
            .unwrap_or_else(|why| panic!("`{text}` is a type: {why:?}"))
    }

    fn variant(name: &str, value: i128) -> ScopedVariant {
        ScopedVariant {
            name: name.to_string(),
            value,
        }
    }

    fn record(name: &str, fields: &[(&str, &str)], scope: TypeScope) -> ScopedRecord {
        ScopedRecord {
            identity: name.to_string(),
            name: name.to_string(),
            fields: fields
                .iter()
                .map(|(id, text)| (id.to_string(), ty(text)))
                .collect(),
            scope,
        }
    }

    /// The types of docs/adr/0101 section 2.7: the enum `Color`, the records
    /// `Customer` and `Line`, and the union `Payment` of `card`, `cash` and
    /// `voucher`, each case's record named in the union's own document.
    fn order_scope() -> TypeScope {
        let mut colors = TypeScope::default();
        colors.enums.insert(
            "Color".to_string(),
            ScopedEnum {
                name: "paint_color".to_string(),
                variants: vec![variant("red", 0), variant("green", 1), variant("blue", 2)],
                underlying: SceType::Uint8,
                strict: true,
            },
        );
        let mut cases = TypeScope::default();
        cases.records.insert(
            "Card".to_string(),
            record(
                "order_card",
                &[("last4", "string[4]"), ("amount", "uint32")],
                TypeScope::default(),
            ),
        );
        cases.records.insert(
            "Voucher".to_string(),
            record(
                "order_voucher",
                &[("voucher_code", "bytes[4]")],
                TypeScope::default(),
            ),
        );
        let mut scope = colors.clone();
        scope.records.insert(
            "Customer".to_string(),
            record(
                "order_customer",
                &[("full_name", "string[24]"), ("vip", "bool")],
                TypeScope::default(),
            ),
        );
        scope.records.insert(
            "Line".to_string(),
            record(
                "order_line",
                &[
                    ("sku", "string[16]"),
                    ("quantity", "uint16"),
                    ("color", "enum:Color"),
                    ("note", "optional<string[8]>"),
                ],
                colors,
            ),
        );
        scope.unions.insert(
            "Payment".to_string(),
            ScopedUnion {
                identity: "order_payment".to_string(),
                name: "order_payment".to_string(),
                tag: "method".to_string(),
                tag_enum: "Method".to_string(),
                cases: vec![
                    ScopedCase {
                        variant: variant("card", 0),
                        record: Some("Card".to_string()),
                    },
                    ScopedCase {
                        variant: variant("cash", 1),
                        record: None,
                    },
                    ScopedCase {
                        variant: variant("voucher", 2),
                        record: Some("Voucher".to_string()),
                    },
                ],
                scope: cases,
            },
        );
        scope
    }

    fn shape(text: &str) -> CborShape {
        CborShape::of(&ty(text), &order_scope())
    }

    fn uint(bits: u32) -> CborShape {
        CborShape::Int {
            bits,
            signed: false,
        }
    }

    #[test]
    fn a_number_a_text_and_a_list_are_planned_as_their_type_states() {
        assert_eq!(
            shape("int8"),
            CborShape::Int {
                bits: 8,
                signed: true
            }
        );
        assert_eq!(shape("uint64"), uint(64));
        assert_eq!(shape("float32"), CborShape::Float { bits: 32 });
        assert_eq!(shape("bool"), CborShape::Bool);
        assert_eq!(shape("string[8]"), CborShape::Text { max: 8 });
        // An exact length is an entry's own, never a type's.
        assert_eq!(
            shape("bytes[4]"),
            CborShape::Bytes {
                max: 4,
                exact: false
            }
        );
        // An absent element of a list is written as `null`.
        assert_eq!(
            shape("list<optional<uint16>>[3]"),
            CborShape::List {
                elem: Box::new(CborShape::Nullable(Box::new(uint(16)))),
                max: 3,
                min: 0,
            }
        );
        assert_eq!(
            shape("list<list<uint8>[2]>[1..2]"),
            CborShape::List {
                elem: Box::new(CborShape::List {
                    elem: Box::new(uint(8)),
                    max: 2,
                    min: 0,
                }),
                max: 2,
                min: 1,
            }
        );
        assert_eq!(
            shape("enum:Color"),
            CborShape::Enum(CborEnum {
                alias: "Color".to_string(),
                carrier: SceType::Uint8,
                closed: true,
            })
        );
        assert_eq!(shape("list<uint8>[4]").listed_record(), None);
    }

    #[test]
    fn a_record_is_planned_with_its_fields_in_the_order_of_their_encoded_keys() {
        let CborShape::Record(customer) = shape("record:Customer") else {
            panic!("a record")
        };
        assert_eq!(
            customer,
            CborRecord {
                alias: "Customer".to_string(),
                name: "order_customer".to_string(),
                fields: vec![
                    CborField {
                        id: "full_name".to_string(),
                        ty: ty("string[24]"),
                        optional: false,
                        shape: CborShape::Text { max: 24 },
                    },
                    CborField {
                        id: "vip".to_string(),
                        ty: ty("bool"),
                        optional: false,
                        shape: CborShape::Bool,
                    },
                ],
                // `vip` first: its encoding is shorter (docs/adr/0101, section
                // 2.8).
                by_key: vec![1, 0],
            }
        );
        // A field's enum is read in the record's own document, and an optional
        // field is planned as its element, its key left out where it is absent.
        let line = shape("list<record:Line>[4]");
        let line = line.listed_record().expect("a list of records");
        let written: Vec<&str> = line.fields_by_key().map(|f| f.id.as_str()).collect();
        assert_eq!(written, ["sku", "note", "color", "quantity"]);
        let color = &line.fields[2];
        assert!(matches!(&color.shape, CborShape::Enum(e) if e.alias == "Color"));
        let note = &line.fields[3];
        assert!(note.optional);
        assert_eq!(note.ty, ty("optional<string[8]>"));
        assert_eq!(note.shape, CborShape::Text { max: 8 });
    }

    #[test]
    fn a_union_is_planned_with_each_case_by_its_variant_and_its_record() {
        let CborShape::Union(payment) = shape("union:Payment") else {
            panic!("a union")
        };
        assert_eq!(payment.alias, "Payment");
        assert_eq!(payment.name, "order_payment");
        let cases: Vec<(&str, i128, Option<&str>)> = payment
            .cases
            .iter()
            .map(|c| {
                (
                    c.variant.as_str(),
                    c.number,
                    c.record.as_ref().map(|r| r.name.as_str()),
                )
            })
            .collect();
        assert_eq!(
            cases,
            [
                ("card", 0, Some("order_card")),
                ("cash", 1, None),
                ("voucher", 2, Some("order_voucher")),
            ]
        );
        let card = payment.cases[0].record.as_ref().expect("a record");
        let written: Vec<&str> = card.fields_by_key().map(|f| f.id.as_str()).collect();
        assert_eq!(written, ["last4", "amount"]);
        // An optional union, as a list's element, is the union or `null`.
        assert_eq!(
            shape("list<optional<union:Payment>>[2]"),
            CborShape::List {
                elem: Box::new(CborShape::Nullable(Box::new(CborShape::Union(payment)))),
                max: 2,
                min: 0,
            }
        );
    }

    #[test]
    fn what_the_scope_does_not_resolve_is_named_and_a_record_that_holds_itself_ends() {
        assert_eq!(
            shape("record:Nope"),
            CborShape::Unresolved {
                what: "Nope".to_string()
            }
        );
        assert_eq!(
            shape("list<enum:Nope>[2]"),
            CborShape::List {
                elem: Box::new(CborShape::Unresolved {
                    what: "Nope".to_string()
                }),
                max: 2,
                min: 0,
            }
        );
        // A case whose record the union's document does not resolve.
        let mut scope = order_scope();
        if let Some(payment) = scope.unions.get_mut("Payment") {
            payment.scope.records.remove("Voucher");
        }
        assert_eq!(
            CborShape::of(&ty("union:Payment"), &scope),
            CborShape::Unresolved {
                what: "Voucher".to_string()
            }
        );
        // A record that holds itself through a list of itself.
        let inner = record(
            "node",
            &[("next", "list<record:Node>[2]")],
            TypeScope::default(),
        );
        let mut own = TypeScope::default();
        own.records.insert("Node".to_string(), inner);
        let mut scope = TypeScope::default();
        scope.records.insert(
            "Node".to_string(),
            record("node", &[("next", "list<record:Node>[2]")], own),
        );
        let CborShape::Record(node) = CborShape::of(&ty("record:Node"), &scope) else {
            panic!("a record")
        };
        assert_eq!(
            node.fields[0].shape,
            CborShape::List {
                elem: Box::new(CborShape::Unresolved {
                    what: "Node".to_string()
                }),
                max: 2,
                min: 0,
            }
        );
    }

    #[test]
    fn a_map_of_text_keys_is_written_shortest_key_first_then_by_its_bytes() {
        assert_eq!(
            text_key_order(["author", "seq", "id", "zz", "ab"]),
            vec![4, 2, 3, 1, 0]
        );
        // Past 23 bytes a text's head is two bytes: still the shorter first.
        let long = "a".repeat(24);
        let short = "b".repeat(23);
        assert_eq!(text_key_order([long.as_str(), short.as_str()]), vec![1, 0]);
        assert!(text_key_order(std::iter::empty::<&str>()).is_empty());
    }

    #[test]
    fn an_entry_is_planned_as_its_present_type_and_states_its_exact_length() {
        let label = crate::DocumentLabel {
            identifier: "t",
            diagnostic_label: ".scxml-fixture",
        };
        let xml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                       sce:kind="codec" sce:encoding="cbor" sce:codec-format="2" name="t">
                 <datamodel>
                 <data id="raw" sce:type="bytes" sce:direction="in"/>
                 <data id="tag" sce:type="bytes" sce:key="3" sce:length="4" sce:direction="out"/>
                 <data id="blob" sce:type="optional&lt;bytes[8]&gt;" sce:key="1" sce:direction="out"/>
                 <data id="n" sce:type="uint16" sce:key="2" sce:direction="out"/>
               </datamodel></scxml>"#;
        let Ok(Some(crate::forge::model::ForgeDocument::Codec(m))) =
            crate::forge::parser::parse_forge(xml, label)
        else {
            panic!("the fixture is a codec document")
        };
        let items = cbor_items(&m, &TypeScope::default());
        let planned: Vec<(&str, bool, CborShape)> = items
            .iter()
            .map(|item| (item.entry.id.as_str(), item.required, item.value.clone()))
            .collect();
        assert_eq!(
            planned,
            vec![
                (
                    "tag",
                    true,
                    CborShape::Bytes {
                        max: 4,
                        exact: true
                    }
                ),
                (
                    "blob",
                    false,
                    CborShape::Bytes {
                        max: 8,
                        exact: false
                    }
                ),
                ("n", true, uint(16)),
            ]
        );
        assert_eq!(entry_key_order(&m.cbor_entries), vec![1, 2, 0]);
    }
}
