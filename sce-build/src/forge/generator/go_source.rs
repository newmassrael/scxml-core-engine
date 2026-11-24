// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Go printer of a codec's encode plan (docs/adr/0081 §2, step 4).
//!
//! A Go value presents its elements through methods, the methods of an
//! interface `{Codec}Source` that names what a source of the codec is, and
//! holds them in unexported fields, so a value outside its package is made by
//! one constructor, `Make{Codec}`, by its decode, or as its zero value, and is
//! read through its methods. Its encode is a function of the codec's package,
//! `EncodeSource`, over any source: it reads each element the source presents
//! and derives every other element of the frame from them, as the plan says,
//! never from the value (docs/adr/0058 §1).
//!
//! A byte string or a text is presented as the runtime's `codec.Bytes` or
//! `codec.Text` of a bound type the codec names for the element,
//! `{Codec}{Element}Bound`: the abstract type of §2 for Go, a struct with an
//! unexported field made only by `codec.MakeBytes` or `codec.MakeText`, which
//! check the bound where the value is made, so the encode over it refuses
//! nothing the bound decides.
//!
//! An element a gate decides, and a length the value states only while its
//! content is absent (docs/adr/0058 §1), is presented as the runtime's
//! `optional.Of` of what it holds. The encode derives each flag such an
//! element's presence says, and folds it into its carrier in place of the
//! bits the value presents.
//!
//! A body of another printed codec is presented, held and made as that
//! codec's source interface, so a value is made of any source of the body, and
//! is written by that codec's own `EncodeSource`. The encode counts a body
//! over the runtime's `codec.CountingSink` where a length measures its frame
//! or where its encode can refuse, before the first byte. A list is the
//! runtime's `codec.List` of its entries' source interface, held to its bound
//! and walked through an `iter.Seq`. A number the encode
//! derives into a field is answered by a method of the value,
//! `Wire{Field}`, as the frame carries it; one that counts a body handed a
//! flag input of the codec's own takes the input, as the encode does, and a
//! carrier a counted body is handed a derived flag of is answered by a
//! method of its own, its derived flags folded in. The encode counts a body
//! once every carrier is folded, so the body is handed each flag as the
//! frame carries it.
//!
//! A variant is presented as the arm the value holds, `Body()`, a value of
//! the interface `{Codec}Variant`, which only the arm types the codec's
//! package declares satisfy: each holds a source of its arm's codec, and the
//! catch-all the tag it holds as `{Codec}UnknownTag`, made only of a tag no
//! arm names (docs/adr/0065 §6 R3). The encode derives the tag from the arm
//! and writes the arm by its codec's `EncodeSource`; a parent derives the
//! bits of its carrier the arm of an embedded body's variant fills.
//!
//! A group of fields one flag of the codec's own decides, or a disjunction of
//! such flags (docs/adr/0065 §6 R4b, R4c), is presented whole: the struct of
//! its one arm, or an interface its arm types satisfy, as the optional type
//! of it where the gates allow no arm. The encode binds each member to a
//! local of its own from the arm the source presents and derives every gate
//! from it; two lists of one count are the runtime's `codec.Aligned` of them
//! in the arm that holds both (R5).
//!
//! An encode that can refuse returns the refusal the plan's totality names,
//! the runtime's sentinel of its name (`codec.ErrPresentIfMismatch`,
//! `codec.ErrInconsistentSource`, `codec.ErrLengthOutOfRange`), before it
//! writes the first byte; one that cannot returns only the sink's error.
//!
//! The printer reads the model only through the plan and lowers its forms one
//! landing at a time (§6): [`unlowered`] names the first form a codec's plan
//! uses that it does not print, and such a codec keeps the template's encode
//! whole, so no codec has two encodes.

use super::printed_codec::{DerivedNumber, PrintedMade, PrintedPart, PrintedRebuild, Reading};
use super::*;
use crate::forge::codec_roles::{ElementId, GateGroup, GateVar, GroupArm};
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_plan::{
    Absent, Arm, Clause, CodecPlan, Derive, Element, EncodePlan, EntryId, Guard, HandedFrom,
    ListWrite, Measure, Measured, Narrow, Number, NumberInto, Offset, ParentTag, Presence,
    PresenceFlag, Value, Write,
};
use std::collections::{BTreeMap, BTreeSet};

/// The first form of `plan` the Go printer does not print, as the clause that
/// names it, or `None` where it prints the codec whole.
pub(super) fn unlowered(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Option<String> {
    unlowered_forms(plan, imports).into_iter().next()
}

/// Every form of `plan` the Go printer does not print, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole.
///
/// It prints a positional codec whose elements are integers, flag sets,
/// enums, byte strings, texts, and bodies and lists of codecs it prints, held
/// to no law and walked with no flag handed to an entry, a chain setting on
/// each entry but the last the flag it ends on, each always
/// there or decided by a gate on a flag of the codec's own carriers or on an
/// input the encode is handed, the integers written whole, in bytes they share
/// with other fields, or as a variable-length integer of 16, 32 or 64 bits,
/// the byte strings and texts written to the end of the frame or by a length,
/// and a body handed each flag input it does not say from a carrier or an
/// input of this codec's, and whose variant's arms are codecs it prints,
/// handed as a body is; whose groups of fields one flag of its own, or a
/// disjunction of such flags, decides hold members of those kinds, two lists
/// of one count as one pair; whose every derived element is a length of byte
/// strings or texts, the count of a list, or the encoded length of a body,
/// held in a field or in a
/// flag of a carrier the value presents, its `sce:length-arith` never taking
/// it below zero, a flag the presence of one element or of one group says, a
/// flag of a disjunction a group's arm gives, the tag of the arm the value
/// holds, or the bits of a carrier the arm of an embedded body's variant
/// fills; and whose
/// encode refuses what its totality names of these, a refusal of a body it
/// holds among them. A CBOR map and a content-line component keep their own
/// templates' encodes. `imports` are the codec's own, which say whether a
/// codec it holds is printed ([`ImportContext::printed`]).
pub(super) fn unlowered_forms(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    match plan {
        CodecPlan::Positional(plan) => positional_forms(plan, imports),
        CodecPlan::Cbor(_) => vec!["a CBOR map".to_string()],
        CodecPlan::ContentLine(_) => vec!["a content-line component".to_string()],
    }
}

/// The import of the codec `plan` holds as `alias`, where the Go backend
/// prints it from its plan: an import read as the Go backend generates it
/// says so by what a reader knows of it.
fn printed_import<'i>(imports: &'i [ImportContext], alias: &str) -> Option<&'i ImportContext> {
    imports
        .iter()
        .find(|import| import.alias == alias && import.printed.is_some())
}

/// [`unlowered_forms`] of a positional codec.
fn positional_forms(plan: &EncodePlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    let m = plan.model;
    let mut forms = Vec::new();
    // A variant is presented as the arm the value holds, each arm a source
    // of its codec, and written by that codec's own `EncodeSource`, which a
    // codec the Go backend prints has, handed each input it takes and does
    // not say from a carrier or an input of this codec's. An arm a build
    // feature decides (docs/adr/0072) is an arm of every Go build, as the
    // template keeps it.
    for arm in &plan.arms {
        if printed_import(imports, arm.codec).is_none() {
            forms.push(format!(
                "a variant: the arm `{}` is a codec the Go backend does not print",
                arm.codec
            ));
        }
        if plan
            .handed
            .get(arm.codec)
            .is_some_and(|handed| handed.iter().any(|h| h.from.is_none()))
        {
            forms.push(format!(
                "a variant: the arm `{}` is handed an input its parent binds nothing to",
                arm.codec
            ));
        }
    }
    // An input the value says is read off it by a parent; one it does not say
    // is a parameter of the encode.
    for said in &plan.said {
        forms.push(format!(
            "input '{}': a flag input the value says",
            said.input.name
        ));
    }
    // An element a gate decides, and a length the value states while its
    // content is absent, are presented as the optional type of what they
    // hold, whatever they hold.
    for element in &plan.elements {
        forms.extend(value_forms(plan, imports, element.field, &element.value));
    }
    // A group of fields a flag of the codec's own decides, or a disjunction
    // of such flags, is presented whole, each arm holding its members as an
    // element holds what it holds (docs/adr/0065 §6 R4b, R4c). A group an
    // input decides is one of an input the value says; one a chain's entries
    // decide is not printed yet.
    for group in plan.groups() {
        for gate in &group.gates {
            match gate {
                GateVar::Flag { .. } => {}
                GateVar::Input(name) => forms.push(format!(
                    "a group the flag input '{name}' decides, which the value says"
                )),
                GateVar::Member { chain, .. } => {
                    forms.push(format!("field '{chain}': a group a chain's entries decide"))
                }
            }
        }
        for id in group.fields() {
            let field = m
                .fields
                .iter()
                .find(|f| f.id == id)
                .expect("a member of a group is a field of the codec");
            forms.extend(value_forms(plan, imports, field, &plan.value(field)));
        }
    }
    let folded = folded_carriers(plan);
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            // A number into a field, or into a flag of a carrier. The encoded
            // length of a body is counted once every carrier is folded, so the
            // body is handed each flag as the encode writes it.
            (_, Derive::Number(number)) => {
                forms.extend(number_form(&derived.element, number));
            }
            (ElementId::Flag { .. }, Derive::Presence(presence)) => {
                forms.extend(presence_form(&derived.element, presence));
            }
            // The flag a chain's entry is closed by, which the walk of the
            // chain sets on each entry as it writes it.
            (ElementId::Flag { .. }, Derive::ChainContinuation) => {}
            // The tag of the arm the value holds, into a field of an integer
            // or a flag of a carrier; and a carrier's bits the arm of an
            // embedded body's variant fills, whose body the elements above
            // hold to a printed codec.
            (ElementId::Field(id), Derive::VariantTag) => {
                if !m
                    .fields
                    .iter()
                    .any(|f| &f.id == id && f.sce_type.int_bit_width().is_some())
                {
                    forms.push(format!(
                        "element '{id}': a variant's tag held in a field of no integer"
                    ));
                }
            }
            (ElementId::Flag { .. }, Derive::VariantTag | Derive::ParentTag(_)) => {}
            // A flag of a disjunction that decides a group: what the arm the
            // value holds gives it (docs/adr/0065 §6 R4c).
            (ElementId::Flag { .. }, Derive::ArmFlag { .. }) => {}
            (element, _) => forms.push(format!(
                "element '{element}': an element the encode derives other than a length, \
                 a presence flag, a chain's end flag or a variant's tag"
            )),
        }
    }
    // A flag the encode derives is folded into its carrier as the value
    // presents it, which the value then presents whole.
    for &carrier in &folded {
        if element_of(plan, carrier).map(|e| e.presence) != Some(Presence::Always) {
            forms.push(format!(
                "field '{carrier}': a carrier of a derived flag the value does not present whole"
            ));
        }
    }
    // A gate the encode cannot derive the presence from is a check the value
    // is held to before the first byte, as the encode writes the carriers it
    // reads.
    for field in &plan.gate_checks {
        forms.extend(gate_form(plan, field));
    }
    if !plan.input_checks.is_empty() {
        forms.push("an input a held codec says, held to what the codec is handed".to_string());
    }
    let wholes = derived_fields(plan);
    for write in &plan.program {
        match write {
            Write::Whole(field) | Write::Tail { field, .. } => {
                forms.extend(unread_form(plan, &wholes, field));
            }
            Write::Byte { pieces, .. } => {
                for (field, _) in pieces {
                    forms.extend(unread_form(plan, &wholes, field));
                }
            }
            Write::Streamed { field, guard, list } => {
                forms.extend(unread_form(plan, &wholes, field));
                if *guard == Guard::Gate {
                    forms.extend(gate_form(plan, field));
                }
                // A walk of each entry in turn, which sets on each entry but
                // the last the flag the chain ends on and hands each entry
                // what the one before it was; a chain that holds a bare
                // entry's place or is held to a law is not printed yet.
                if let Some(walk) = list {
                    if walk.slots.is_some() || walk.lawful {
                        forms.push(format!(
                            "field '{}': a chain that holds a bare entry's place or is held \
                             to a law",
                            field.id
                        ));
                    }
                    continue;
                }
                match &field.bit_size {
                    BitSize::Fixed { .. } => {
                        let whole =
                            BitSpan::of(field, m.default_endian).is_some_and(|span| span.whole());
                        if !whole {
                            forms.push(format!(
                                "field '{}': a streamed field of part of its bytes",
                                field.id
                            ));
                        }
                    }
                    BitSize::Vle {
                        width_bits: 16 | 32 | 64,
                    }
                    | BitSize::Tail
                    | BitSize::LengthRef => {}
                    BitSize::Vle { width_bits } => forms.push(format!(
                        "field '{}': a variable-length integer of {width_bits} bits",
                        field.id
                    )),
                    // A body or a list, which the elements above name.
                    BitSize::Embed | BitSize::Repeat { .. } | BitSize::TlvChain { .. } => {}
                }
            }
            // The arm the value holds, which the arms above hold to printed
            // codecs.
            Write::Variant => {}
        }
    }
    forms.dedup();
    forms
}

/// What of `field`, an element the value presents or a member of a group,
/// which holds `value`, the printer does not print.
fn value_forms(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
    field: &CodecField,
    value: &Value<'_>,
) -> Vec<String> {
    let id = &field.id;
    let mut forms = Vec::new();
    match value {
        Value::Scalar(t) if t.int_bit_width().is_some() => {}
        Value::Scalar(SceType::Enum(_)) => {}
        Value::Scalar(t) => forms.push(format!("field '{id}': a `{}` field", t.as_attr())),
        // Written whole, to the end of the frame or by its length.
        Value::Bytes(_) | Value::Text(_) => {
            if !matches!(field.bit_size, BitSize::Tail | BitSize::LengthRef) {
                forms.push(format!(
                    "field '{id}': a byte string or a text of a fixed size"
                ));
            }
        }
        // Written by its codec's own `EncodeSource`, which a codec the Go
        // backend prints has, handed each input it takes and does not say
        // from a carrier or an input of this codec's.
        Value::Body { codec } => {
            if printed_import(imports, codec).is_none() {
                forms.push(format!(
                    "field '{id}': a body of `{codec}`, which the Go backend does not print"
                ));
            }
            if plan
                .handed
                .get(*codec)
                .is_some_and(|handed| handed.iter().any(|h| h.from.is_none()))
            {
                forms.push(format!(
                    "field '{id}': a body handed an input its parent binds nothing to"
                ));
            }
        }
        // Walked, each entry written by its codec's own `EncodeSource`,
        // handed as a body is.
        Value::List { codec, law, .. } => {
            if printed_import(imports, codec).is_none() {
                forms.push(format!(
                    "field '{id}': a list of `{codec}`, which the Go backend does not print"
                ));
            }
            if !law.is_none() {
                forms.push(format!("field '{id}': a list held to a law"));
            }
            // An input its parent binds nothing to is one the walk of a
            // chain hands each entry from the entry before.
            let supplied = |input: &str| {
                list_walk(plan, id)
                    .is_some_and(|walk| walk.previous.iter().any(|(i, _)| i == input))
            };
            if plan.handed.get(*codec).is_some_and(|handed| {
                handed
                    .iter()
                    .any(|h| h.from.is_none() && !supplied(&h.input))
            }) {
                forms.push(format!(
                    "field '{id}': a list handed an input neither its parent nor its walk \
                     binds"
                ));
            }
        }
    }
    forms
}

/// What of the derived number of `element` the printer does not print: it
/// prints a length of the byte strings and texts it measures, each of which
/// that is there says the same number or the value is refused, its
/// `sce:length-arith` never taking the number below zero, into a field that
/// holds nothing else, refused where it may not fit, or into a flag of a
/// carrier; where nothing it measures is there the field holds zero, gated
/// beside its content, what the value states, or what the arm of a group
/// without the content holds (docs/adr/0065 §6 R4b).
fn number_form(element: &ElementId, number: &Number<'_>) -> Vec<String> {
    let mut forms = Vec::new();
    for measure in &number.measures {
        match &measure.of {
            Measured::Bytes {
                offset: Offset::Less { held: false, .. },
                ..
            } => forms.push(format!(
                "element '{element}': a length its arithmetic may take below zero"
            )),
            // The bytes of a byte string or a text, or the entries of a list.
            Measured::Bytes { .. } | Measured::Entries { .. } => {}
            // Counted by the body's own encode over a sink that keeps nothing.
            Measured::Encoded { .. } => {}
            Measured::Says { .. } | Measured::Start { .. } | Measured::ArmSays { .. } => {
                forms.push(format!("element '{element}': a flag a held codec says"));
            }
        }
    }
    forms
}

/// What of the presence flag `element` the printer does not print: it prints
/// a flag the presence of one element decides, or of the members one group
/// holds together, the first of which says it.
fn presence_form(element: &ElementId, presence: &PresenceFlag<'_>) -> Option<String> {
    (presence.of.len() != 1 && !presence.together)
        .then(|| format!("element '{element}': a flag the presence of several fields decides"))
}

/// What of the gate of `field`, a field written where its gate holds or held
/// to it, the printer does not print: it tests a flag of the codec's own
/// carriers or an input the encode is handed.
fn gate_form(plan: &EncodePlan<'_>, field: &CodecField) -> Option<String> {
    let gate = field.present_if.as_ref()?;
    plan.clauses(gate)
        .iter()
        .any(|clause| matches!(clause, Clause::Member { .. }))
        .then(|| format!("field '{}': a gate on a chain's entries", field.id))
}

/// The carriers the encode folds a flag it derives into, by field id: a flag
/// the presence of an element says, one that holds a length, the tag of the
/// arm the value holds, or the bits the arm of an embedded body's variant
/// fills.
fn folded_carriers<'p>(plan: &'p EncodePlan<'_>) -> BTreeSet<&'p str> {
    plan.derivations
        .iter()
        .filter_map(|derived| match (&derived.element, &derived.by) {
            (
                ElementId::Flag { field, .. },
                Derive::Presence(_)
                | Derive::Number(_)
                | Derive::VariantTag
                | Derive::ParentTag(_)
                | Derive::ArmFlag { .. },
            ) => Some(field.as_str()),
            _ => None,
        })
        .collect()
}

/// The fields of `plan` whose value the encode derives whole: a number, or the
/// tag of the arm of the codec's variant the value holds.
fn derived_fields(plan: &EncodePlan<'_>) -> BTreeSet<String> {
    plan.derivations
        .iter()
        .filter(|derived| matches!(derived.by, Derive::Number(_) | Derive::VariantTag))
        .filter_map(|derived| match &derived.element {
            ElementId::Field(id) => Some(id.clone()),
            _ => None,
        })
        .collect()
}

/// The fields of `plan` whose value the encode derives as a number.
fn derived_numbers(plan: &EncodePlan<'_>) -> BTreeSet<String> {
    plan.derivations
        .iter()
        .filter(|derived| matches!(derived.by, Derive::Number(_)))
        .filter_map(|derived| match &derived.element {
            ElementId::Field(id) => Some(id.clone()),
            _ => None,
        })
        .collect()
}

/// Whether the printer cannot read `field` as a write reads it: the value
/// presents it alone or in a group, or the encode derives it whole.
fn unread_form(
    plan: &EncodePlan<'_>,
    wholes: &BTreeSet<String>,
    field: &CodecField,
) -> Option<String> {
    (element_of(plan, &field.id).is_none()
        && !wholes.contains(&field.id)
        && !grouped_field(plan, &field.id))
    .then(|| {
        format!(
            "field '{}': a field the value neither presents nor the encode derives whole",
            field.id
        )
    })
}

/// The element of `plan` the value presents as `id`, if it presents one.
fn element_of<'p, 'm>(plan: &'p EncodePlan<'m>, id: &str) -> Option<&'p Element<'m>> {
    plan.elements.iter().find(|e| e.field.id == id)
}

/// Whether the Go value of `plan`, a plan [`unlowered`] answers `None` for,
/// holds no field `id`: the encode derives it (docs/adr/0081 §3) and the
/// value does not state it while its content is absent.
pub(super) fn derived_field(plan: &EncodePlan<'_>, id: &str) -> bool {
    derived_fields(plan).contains(id) && element_of(plan, id).is_none()
}

/// Whether the field `id` of `plan`, a plan [`unlowered`] answers `None` for,
/// is a member of a gate group, which the Go value holds whole in a field of
/// the group's own ([`GoSource::groups`]) and not in a field of its own.
pub(super) fn grouped_field(plan: &EncodePlan<'_>, id: &str) -> bool {
    plan.groups().iter().any(|group| group.contains(id))
}

/// The unexported field a Go value printed from its plan holds the element
/// `id` in: the name of its method, the element's Go field name, with its
/// first letter lowered. Lowering the first letter of distinct exported names
/// keeps them distinct. A name whose field would be a Go keyword is refused in
/// the document by this same fold (`Case::LowerPascal` in the Go column of a
/// codec member's spellings, `scxml_identifier::spellings_of`), so the name
/// needs no escape.
pub(super) fn member(id: &str) -> String {
    crate::reader_names::Case::LowerPascal.spell(id)
}

/// The local a rebuild reads a bounded element it made through
/// [`PrintedRebuild::made`] from.
pub(crate) const GO_REBUILT_ELEMENT: &str = "sceMade";

/// The local a rebuild holds the value assigned to a byte string or a text
/// in, which the call of [`PrintedRebuild::made`] makes the bounded value of.
pub(crate) const GO_REBUILT_VALUE: &str = "sceValue";

/// The runtime's bounded type of a byte string or, where `text`, a text:
/// `codec.Bytes` or `codec.Text`, and the stem of the runtime's functions over
/// it (`codec.MakeBytes`, `codec.OptionalText`).
fn bounded_kind(text: bool) -> &'static str {
    if text {
        "Text"
    } else {
        "Bytes"
    }
}

/// What a reader in another package knows of the codec `plan`, whose value's
/// struct is `struct_name`, which the Go backend prints ([`unlowered`] answers
/// `None` for it), in the shape every printer gives it (docs/adr/0081 step 4):
/// the elements its value presents, each by its method, in the order
/// `Make{Codec}` takes them, a group and the arm of the variant whole; and
/// each number its encode derives into a field, the tag of the arm the value
/// holds among them, by the method of the value that answers it
/// ([`wire_method`]), answered to a reader where the method takes none of the
/// inputs the encode is handed. A procedure, a validator and an algorithm that
/// hold a value of the codec read and make it through this
/// ([`PrintedCodec::go_read`], [`PrintedCodec::go_rebuilt`]), so none spells a
/// printed codec's interface of its own.
pub(crate) fn printed_codec(plan: &EncodePlan<'_>, struct_name: &str) -> PrintedCodec {
    // In declaration order, each group where its first member is declared,
    // then the arm of the variant, as `render` declares them.
    let groups = go_groups(plan, struct_name);
    let mut held_groups: BTreeSet<usize> = BTreeSet::new();
    let mut parts: Vec<PrintedPart> = Vec::new();
    for field in &plan.model.fields {
        let id = field.id.clone();
        if let Some(element) = element_of(plan, &id) {
            parts.push(PrintedPart::Element {
                accessor: filters::to_pascal_case(id.clone()),
                made: PrintedMade::of(&element.value, element.presence.optional()),
                id,
            });
        } else if let Some(at) = groups.iter().position(|g| g.group.contains(&id)) {
            if held_groups.insert(at) {
                parts.push(PrintedPart::Whole {
                    accessor: groups[at].method.clone(),
                });
            }
        }
    }
    if !plan.arms.is_empty() {
        parts.push(PrintedPart::Whole {
            accessor: VARIANT_METHOD.to_string(),
        });
    }
    let mut numbers: Vec<DerivedNumber> = wire_numbers(plan)
        .into_iter()
        .map(|(id, number)| DerivedNumber {
            id: id.to_string(),
            by: wire_method(id),
            answered: wire_inputs(plan, number).is_empty(),
        })
        .collect();
    numbers.extend(wire_tags(plan).into_iter().map(|id| DerivedNumber {
        id: id.to_string(),
        by: wire_method(id),
        answered: true,
    }));
    PrintedCodec { parts, numbers }
}

impl PrintedCodec {
    /// The Go read of the field `id` of the value `held`: an element the
    /// value presents by its method, a byte string or a text as the slice or
    /// the string its bounded value holds, the optional type of them where a
    /// gate decides it, as the template's value held it; a number the encode
    /// derives into a field by the method that derives it as the encode does
    /// ([`wire_method`]), since the value holds no field for it (docs/adr/0081
    /// §3), and a length the value states while its content is absent as the
    /// number the frame carries, which that method answers too. `None` where a
    /// reader may not name the field ([`PrintedCodec::reads`]).
    pub(crate) fn go_read(&self, held: &str, id: &str) -> Option<String> {
        Some(match self.reading(id)? {
            Reading::Number(method) => format!("{held}.{method}()"),
            Reading::Element { accessor, made } => {
                let call = format!("{held}.{accessor}()");
                match made {
                    PrintedMade::Bounded {
                        text,
                        optional: true,
                    } => format!("codec.Optional{}({call})", bounded_kind(text)),
                    PrintedMade::Bounded { text: true, .. } => format!("{call}.String()"),
                    PrintedMade::Bounded { text: false, .. } => format!("{call}.Bytes()"),
                    PrintedMade::AsGiven | PrintedMade::Unmade { .. } => call,
                }
            }
        })
    }

    /// The Go rebuild of the value `held`, made by `make` (the qualified
    /// `Make{Codec}` of the codec whose struct is `struct_name`) in the
    /// package `package` names, with the element `element` assigned `value`
    /// and everything else `Make{Codec}` takes read by its method
    /// ([`PrintedCodec::rebuilt_by`]). A byte string or a text is assigned as
    /// the slice or the string the expression gives, made into its bounded
    /// value by the runtime's constructor over the bound type the codec's
    /// package names for it, which refuses one outside the bound or a text that
    /// is not UTF-8 ([`PrintedRebuild::made`]); one a gate decides, as the
    /// optional type of them, by the constructor over that type. Any other
    /// element is handed as the expression gives it, which is the type the
    /// template's value held it as, the optional type where a gate decides it.
    pub(crate) fn go_rebuilt(
        &self,
        held: &str,
        make: &str,
        package: Option<&str>,
        struct_name: &str,
        element: &str,
        value: &str,
    ) -> Result<PrintedRebuild, String> {
        self.rebuilt_by(held, make, element, value, |_, text, optional| {
            let bound = bound_type(struct_name, element);
            let bound = match package {
                Some(package) => format!("{package}.{bound}"),
                None => bound,
            };
            let optional = if optional { "Optional" } else { "" };
            (
                format!(
                    "codec.Make{optional}{}[{bound}]({GO_REBUILT_VALUE})",
                    bounded_kind(text)
                ),
                GO_REBUILT_ELEMENT.to_string(),
            )
        })
    }
}

/// Each number the encode of `plan` derives into a field, by the field's id,
/// in the order of the derivations, each field once.
fn wire_numbers<'p, 'm>(plan: &'p EncodePlan<'m>) -> Vec<(&'p str, &'p Number<'m>)> {
    let mut seen = BTreeSet::new();
    plan.derivations
        .iter()
        .filter_map(|derived| match (&derived.element, &derived.by) {
            (ElementId::Field(id), Derive::Number(number))
                if matches!(number.into, NumberInto::Field { .. }) =>
            {
                Some((id.as_str(), number))
            }
            _ => None,
        })
        .filter(|(id, _)| seen.insert(*id))
        .collect()
}

/// The flag inputs of the codec's own a body `number` counts is handed, in
/// the order the encode is handed them: the value does not hold them, so the
/// method that answers the number takes them, as the encode does.
fn wire_inputs<'p>(plan: &'p EncodePlan<'_>, number: &Number<'_>) -> Vec<&'p str> {
    let handed_to_a_counted_body = |name: &str| {
        number.measures.iter().any(|measure| match &measure.of {
            Measured::Encoded { codec, .. } => plan.handed.get(*codec).is_some_and(|handed| {
                handed.iter().any(
                    |h| matches!(&h.from, Some(HandedFrom::Input { name: from }) if from == name),
                )
            }),
            _ => false,
        })
    };
    plan.given
        .iter()
        .map(|input| input.name.as_str())
        .filter(|name| handed_to_a_counted_body(name))
        .collect()
}

/// The method of a printed Go value that answers the number its encode
/// derives into the field `id`, as the frame carries it: `Wire` and the
/// field's Go name. The value holds no field for a derived number
/// (docs/adr/0081 §3), and a length it states while its content is absent is
/// presented by the method of the field's own name, so the number the frame
/// carries is a method of another name.
pub(crate) fn wire_method(id: &str) -> String {
    format!("Wire{}", filters::to_pascal_case(id.to_string()))
}

/// The method a printed Go value presents the arm of its variant by, which
/// its source interface names too.
const VARIANT_METHOD: &str = "Body";

/// The local the encode reads the arm of the variant the value holds into,
/// once.
const VARIANT_LOCAL: &str = "_rBody";

/// The local the encode derives the tag of the arm the value holds into, a
/// `uint64`.
const ARM_TAG_LOCAL: &str = "_tBody";

/// The interface the Go value of the codec whose struct is `struct_name`
/// holds the arm of its variant as: a sum of the arm types
/// ([`arm_type`]), which only the codec's package declares.
pub(crate) fn variant_type(struct_name: &str) -> String {
    format!("{struct_name}Variant")
}

/// The type of the arm of `struct_name`'s variant whose codec is imported as
/// `codec`, or of the catch-all where `codec` is `None`, its arm named as every
/// backend names it ([`variant_arm_name`]). The one place the type is made:
/// the printer declares the arm by it, the decode makes it, and a parent of
/// the codec, printed or not, names it in a type switch.
pub(crate) fn arm_type(struct_name: &str, codec: Option<&str>) -> String {
    format!("{}{}", variant_type(struct_name), variant_arm_name(codec))
}

/// The fields of `plan` the encode derives the tag of the arm the value holds
/// into, which the value answers by [`wire_method`].
fn wire_tags<'p>(plan: &'p EncodePlan<'_>) -> Vec<&'p str> {
    plan.derivations
        .iter()
        .filter_map(|derived| match (&derived.element, &derived.by) {
            (ElementId::Field(id), Derive::VariantTag) => Some(id.as_str()),
            _ => None,
        })
        .collect()
}

/// The arm of `plan`'s variant as the arm type of `struct_name` names it.
fn plan_arm_type(struct_name: &str, arm: &Arm<'_>) -> String {
    arm_type(struct_name, arm.tag.map(|_| arm.codec))
}

/// The statements, unindented, that derive into [`ARM_TAG_LOCAL`], a
/// `uint64`, the tag of the arm `variant`, a value of `struct_name`'s variant
/// type, holds: the value the document gives a named arm, and the tag the
/// catch-all holds. An arm no case names, which no value of the codec holds,
/// leaves it zero.
fn arm_tag(plan: &EncodePlan<'_>, struct_name: &str, variant: &str) -> String {
    let mut cases = String::new();
    for arm in &plan.arms {
        let ty = plan_arm_type(struct_name, arm);
        cases.push_str(&match arm.tag {
            Some(tag) => format!("case {ty}:\n\t{ARM_TAG_LOCAL} = 0x{tag:X}\n"),
            None => format!("case {ty}:\n\t{ARM_TAG_LOCAL} = uint64(_b.Tag().Value())\n"),
        });
    }
    // The arm is named only where the catch-all reads its tag off it.
    let subject = if plan.arms.iter().any(|arm| arm.tag.is_none()) {
        format!("_b := {variant}.(type)")
    } else {
        format!("{variant}.(type)")
    };
    format!("var {ARM_TAG_LOCAL} uint64\nswitch {subject} {{\n{cases}}}\n")
}

/// The statements, unindented, that or into `local`, a `uint64`, the bits the
/// arm of each embedded body's variant fills (docs/adr/0058 §1): each arm's
/// tag masked to the bits, the catch-all's the tag it holds, and nothing where
/// a gate leaves the body out. Each body is read off `owner` (`src` or the
/// value) by its method.
fn parent_tag_bits(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    tag: &ParentTag<'_>,
    local: &str,
    owner: &str,
) -> String {
    let mut out = format!("var {local} uint64\n");
    for term in &tag.terms {
        let import = printed_import(imports, &term.codec)
            .expect("`unlowered` prints a body of a codec the Go backend prints");
        let call = format!("{owner}.{}()", l.codec_field_id(&term.body.id));
        let gated = element_of(plan, &term.body.id).is_some_and(|e| e.presence.optional());
        let body = if gated {
            format!("{call}.Proven()")
        } else {
            call.clone()
        };
        let mut cases = String::new();
        for arm in &term.arms {
            cases.push_str(&format!(
                "case {}.{}:\n\t{local} |= 0x{:X}\n",
                import.namespace,
                arm_type(&import.type_name, Some(arm.codec.as_str())),
                arm.tag
            ));
        }
        if term.catch_all {
            cases.push_str(&format!(
                "case {}.{}:\n\t{local} |= uint64(_b.Tag().Value()) & 0x{:X}\n",
                import.namespace,
                arm_type(&import.type_name, None),
                term.mask
            ));
        }
        let subject = if term.catch_all {
            format!("_b := {body}.{VARIANT_METHOD}().(type)")
        } else {
            format!("{body}.{VARIANT_METHOD}().(type)")
        };
        let switch = format!("switch {subject} {{\n{cases}}}\n");
        if gated {
            out.push_str(&format!(
                "if {call}.IsPresent() {{\n{}}}\n",
                indented(&switch)
            ));
        } else {
            out.push_str(&switch);
        }
    }
    out
}

/// Where every body that fills the flag of `tag` is one a gate may leave out,
/// the test that one of them is there, each read off `owner` by its method:
/// with none there, nothing says the flag, and the carrier keeps the bits the
/// value presents (docs/adr/0065 §6 R4a), as a decode that read the frame left
/// them. `None` where a body is always there.
fn parent_tag_said(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    tag: &ParentTag<'_>,
    owner: &str,
) -> Option<String> {
    tag.terms
        .iter()
        .map(|term| {
            element_of(plan, &term.body.id)
                .is_some_and(|e| e.presence.optional())
                .then(|| format!("{owner}.{}().IsPresent()", l.codec_field_id(&term.body.id)))
        })
        .collect::<Option<Vec<String>>>()
        .map(|there| there.join(" || "))
}

/// The statement `fold`, unindented, run only where `said`, when there is a
/// test of it.
fn fold_where_said(said: Option<String>, fold: &str) -> String {
    match said {
        None => fold.to_string(),
        Some(there) => format!("if {there} {{\n{}}}\n", indented(fold)),
    }
}

/// The statement, unindented, that folds `bits`, a `uint64`, masked to the
/// bits of the flag `flag` of the carrier `carrier` of `plan`, into `local`,
/// the carrier as the encode writes it, in place of the bits it held there.
fn fold_bits(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    carrier: &str,
    flag: &str,
    bits: &str,
    local: &str,
) -> String {
    let def = flag_def(plan, carrier, flag);
    let width_mask = flag_width_mask(def);
    let mask = width_mask << def.bit;
    let ty = carrier_type(plan, l, carrier);
    format!(
        "{local} = ({local} &^ 0x{mask:X}) | ({ty}({bits} & 0x{width_mask:X}) << {})\n",
        def.bit
    )
}

/// The Go type of the carrier `id` of `plan`.
fn carrier_type(plan: &EncodePlan<'_>, l: &LangCtx, id: &str) -> String {
    plan.model
        .fields
        .iter()
        .find(|f| f.id == id)
        .map(|carrier| l.type_name(&carrier.sce_type).into_owned())
        .expect("a derived flag is a flag of a carrier of the codec")
}

/// The methods of the value `struct_name` that answer each number its encode
/// derives into a field as the frame carries it ([`wire_method`]), read off
/// the elements the value holds as the encode reads them off a source: the
/// first measure always there; where every measure may be absent, the first
/// one there, and where none is, zero, what the value states, or, for a
/// length gated beside its content, no number, as the field is then not on
/// the wire. A value whose measures disagree has no frame, and its encode
/// refuses it; the method answers its first measure.
///
/// A body a number counts is handed its flags as the encode hands them: a
/// flag of a carrier as the frame carries it, folded by the carrier's own
/// method ([`wire_carrier`]) where the encode derives a flag of it, and an
/// input of the codec's own as the method's parameter ([`wire_inputs`]).
fn wire_methods(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> String {
    let mut out = String::new();
    let folded = folded_carriers(plan);
    // A carrier a body is handed a flag of, read off the value as the frame
    // carries it.
    let carrier = |c: &CodecField| {
        if folded.contains(c.id.as_str()) {
            format!("s.{}()", wire_method(&c.id))
        } else {
            format!("s.{}()", l.codec_field_id(&c.id))
        }
    };
    // The carriers whose method a counted body reads, each once.
    let mut carriers: BTreeSet<&str> = BTreeSet::new();
    let groups = go_groups(plan, struct_name);
    let member_types = member_types(plan, imports, l, struct_name);
    let reads = value_reads(plan, l, &groups);
    for (id, number) in wire_numbers(plan) {
        let NumberInto::Field { field, absent, .. } = &number.into else {
            unreachable!("`wire_numbers` keeps a number into a field");
        };
        for measure in &number.measures {
            if let Measured::Encoded { codec, .. } = &measure.of {
                for handed in plan.handed.get(*codec).into_iter().flatten() {
                    if let Some(HandedFrom::Carrier { carrier: c, .. }) = &handed.from {
                        if folded.contains(c.id.as_str()) {
                            carriers.insert(c.id.as_str());
                        }
                    }
                }
            }
        }
        let ty = l.type_name(&field.sce_type).into_owned();
        let method = wire_method(id);
        let parameters: Vec<String> = wire_inputs(plan, number)
            .iter()
            .map(|name| format!("{} byte", input_name(name)))
            .collect();
        let parameters = parameters.join(", ");
        let optional = *absent == Absent::Zero;
        let returned = if optional {
            format!("optional.Of[{ty}]")
        } else {
            ty.clone()
        };
        let wrap = |size: &str| {
            if optional {
                format!("optional.Some({ty}({size}))")
            } else {
                format!("{ty}({size})")
            }
        };
        let measured = |measure: &Measure<'_>, held: &str, indent: &str| {
            held_measure(plan, imports, &carrier, measure, held, indent)
        };
        // The members of groups the number reads, bound off the value first:
        // what it measures, and what the arm without its content holds.
        let mut read_ids: BTreeSet<&str> = number
            .measures
            .iter()
            .map(|measure| held_content(measure).id.as_str())
            .collect();
        read_ids.insert(id);
        let mut body = bind_needed(
            l,
            &groups,
            &member_types,
            &|member| read_ids.contains(member),
            &BTreeSet::new(),
        );
        match number.measures.iter().find(|measure| !measure.gated) {
            Some(anchor) => {
                let held = reads.value(&held_content(anchor).id);
                let (counted, size) = measured(anchor, &held, "\t");
                body.push_str(&counted);
                body.push_str(&format!("\treturn {}\n", wrap(&size)));
            }
            None => {
                for measure in &number.measures {
                    let (counted, size) = measured(measure, "_v", "\t\t");
                    body.push_str(&format!(
                        "\tif _v, ok := {}.Get(); ok {{\n{counted}\t\treturn {}\n\t}}\n",
                        reads.call(&held_content(measure).id),
                        wrap(&size)
                    ));
                }
                body.push_str(&match absent {
                    Absent::Zero => format!("\treturn optional.None[{ty}]()\n"),
                    // What the value states, or what the arm of the group it
                    // holds without the content holds, zero where it holds
                    // neither (docs/adr/0065 §6 R4b).
                    Absent::Stated | Absent::Group => format!(
                        "\tif _s, ok := {}.Get(); ok {{\n\t\treturn _s\n\t}}\n\treturn 0\n",
                        reads.call(id)
                    ),
                    Absent::Never => "\treturn 0\n".to_string(),
                });
            }
        }
        let handed = if parameters.is_empty() {
            ""
        } else {
            " A body it counts is\n\
             // handed the flag inputs the encode is, which the value does not hold."
        };
        out.push_str(&format!(
            "\n// {method} returns the {id} the frame of the value carries, which the\n\
             // encode derives from what it measures (docs/adr/0081 §3).{handed}\n\
             func (s *{struct_name}) {method}({parameters}) {returned} {{\n{body}}}\n"
        ));
    }
    // The tag of the arm the value holds, where a field holds it.
    for id in wire_tags(plan) {
        let field = plan
            .model
            .fields
            .iter()
            .find(|f| f.id == id)
            .expect("a variant's tag is derived into a field of the codec");
        let ty = l.type_name(&field.sce_type).into_owned();
        let method = wire_method(id);
        out.push_str(&format!(
            "\n// {method} returns the {id} the frame of the value carries: the tag of the\n\
             // arm of its variant it holds (docs/adr/0081 §3).\n\
             func (s *{struct_name}) {method}() {ty} {{\n{}\treturn {ty}({ARM_TAG_LOCAL})\n}}\n",
            indented(&arm_tag(
                plan,
                struct_name,
                &format!("s.{VARIANT_METHOD}()")
            ))
        ));
    }
    for id in carriers {
        out.push_str(&wire_carrier(plan, l, imports, struct_name, id));
    }
    out
}

/// The method of the value `struct_name` that answers the carrier `id` as
/// the frame carries it: the bits the value presents, each flag the encode
/// derives folded in in place of the value's, as the encode folds it. A
/// presence flag is set exactly where the element it gates is there, or
/// exactly where it is not for an element gated on the flag's clear state; a
/// number is the first measure always there, or the first one there, and
/// where none is the carrier keeps the value's bits. A value whose measures
/// disagree, or whose number the flag cannot hold, has no frame; the method
/// answers the bits of its first measure.
fn wire_carrier(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
    id: &str,
) -> String {
    let field = plan
        .model
        .fields
        .iter()
        .find(|f| f.id == id)
        .expect("a folded carrier is a field of the codec");
    let ty = l.type_name(&field.sce_type).into_owned();
    let method = wire_method(id);
    // A number held in a flag counts bytes or entries, never a body's frame
    // (`sce:length-from` names a field), so no body is handed a carrier here.
    let presented = |c: &CodecField| format!("s.{}()", l.codec_field_id(&c.id));
    let groups = go_groups(plan, struct_name);
    let member_types = member_types(plan, imports, l, struct_name);
    let reads = value_reads(plan, l, &groups);
    // The members of groups the folds read, and the groups whose arm a flag
    // of a disjunction is read from, bound off the value first.
    let mut read_ids: BTreeSet<&str> = BTreeSet::new();
    let mut read_groups: BTreeSet<usize> = BTreeSet::new();
    for derived in &plan.derivations {
        let ElementId::Flag { field: carrier, .. } = &derived.element else {
            continue;
        };
        if carrier != id {
            continue;
        }
        match &derived.by {
            Derive::Presence(presence) => {
                read_ids.extend(presence.of.first().map(|(gated, _)| gated.id.as_str()));
            }
            Derive::Number(number) => {
                read_ids.extend(
                    number
                        .measures
                        .iter()
                        .map(|measure| held_content(measure).id.as_str()),
                );
            }
            Derive::ArmFlag { group, .. } => {
                read_groups.insert(*group);
            }
            _ => {}
        }
    }
    let mut body = bind_needed(
        l,
        &groups,
        &member_types,
        &|member| read_ids.contains(member),
        &read_groups,
    );
    body.push_str(&format!("\t_c := s.{}()\n", l.codec_field_id(id)));
    for derived in &plan.derivations {
        let ElementId::Flag {
            field: carrier,
            flag,
        } = &derived.element
        else {
            continue;
        };
        if carrier != id {
            continue;
        }
        let def = flag_def(plan, carrier, flag);
        let mask = flag_width_mask(def) << def.bit;
        match &derived.by {
            Derive::Presence(presence) => {
                let (gated, when_clear) =
                    presence.of.first().expect("a presence flag gates a field");
                let there = format!("{}.IsPresent()", reads.call(&gated.id));
                let test = if *when_clear {
                    format!("!{there}")
                } else {
                    there
                };
                body.push_str(&format!(
                    "\t_c &^= 0x{mask:X}\n\
                     \tif {test} {{\n\
                     \t\t_c |= 0x{mask:X}\n\
                     \t}}\n"
                ));
            }
            Derive::Number(number) => {
                let set = |size: &str, indent: &str| {
                    format!(
                        "{indent}_c = (_c &^ 0x{mask:X}) | ({ty}({size}) << {})\n",
                        def.bit
                    )
                };
                match number.measures.iter().find(|measure| !measure.gated) {
                    Some(anchor) => {
                        let (counted, size) = held_measure(
                            plan,
                            imports,
                            &presented,
                            anchor,
                            &reads.value(&held_content(anchor).id),
                            "\t",
                        );
                        body.push_str(&counted);
                        body.push_str(&set(&size, "\t"));
                    }
                    None => {
                        for (i, measure) in number.measures.iter().enumerate() {
                            let (counted, size) =
                                held_measure(plan, imports, &presented, measure, "_v", "\t\t");
                            let keyword = if i == 0 { "\tif" } else { " else if" };
                            body.push_str(&format!(
                                "{keyword} _v, ok := {}.Get(); ok {{\n{counted}{}\t}}",
                                reads.call(&held_content(measure).id),
                                set(&size, "\t\t")
                            ));
                        }
                        body.push('\n');
                    }
                }
            }
            // The tag of the arm the value holds.
            Derive::VariantTag => {
                body.push_str(&indented(&arm_tag(
                    plan,
                    struct_name,
                    &format!("s.{VARIANT_METHOD}()"),
                )));
                body.push_str(&indented(&fold_bits(
                    plan,
                    l,
                    carrier,
                    flag,
                    ARM_TAG_LOCAL,
                    "_c",
                )));
            }
            // The bits the arm of each embedded body's variant fills, or the
            // value's where no body is there to fill them.
            Derive::ParentTag(tag) => {
                let local = format!("_p{}", derived_local(l, &derived.element));
                body.push_str(&indented(&parent_tag_bits(
                    plan, l, imports, tag, &local, "s",
                )));
                body.push_str(&indented(&fold_where_said(
                    parent_tag_said(plan, l, tag, "s"),
                    &fold_bits(plan, l, carrier, flag, &local, "_c"),
                )));
            }
            // What the arm of the group the value holds gives a flag of the
            // disjunction that decides it.
            Derive::ArmFlag { group, gate } => {
                let local = format!("_a{}", derived_local(l, &derived.element));
                body.push_str(&indented(&group_gate_bit(&groups[*group], *gate, &local)));
                body.push_str(&indented(&fold_bits(plan, l, carrier, flag, &local, "_c")));
            }
            // Set by the chain that holds the entry; a frame of the entry
            // alone carries the bit the value presents.
            Derive::ChainContinuation => {}
        }
    }
    format!(
        "\n// {method} returns the {id} the frame of the value carries: its flags as\n\
         // the value presents them, each flag the encode derives folded in\n\
         // (docs/adr/0081 §3).\n\
         func (s *{struct_name}) {method}() {ty} {{\n{body}\treturn _c\n}}\n"
    )
}

/// The content `measure` measures.
fn held_content<'m>(measure: &Measure<'m>) -> &'m CodecField {
    match &measure.of {
        Measured::Bytes { field, .. } => field,
        Measured::Entries { list } => list,
        Measured::Encoded { body, .. } => body,
        _ => unreachable!("`unlowered` prints a length or a count of content only"),
    }
}

/// How the value's own methods read its elements: off the value, each member
/// of a group off the local [`bind_needed`] binds it to.
fn value_reads<'p, 'm>(
    plan: &'p EncodePlan<'m>,
    l: &'p LangCtx,
    groups: &[GoGroup<'_>],
) -> SourceReads<'p, 'm> {
    SourceReads {
        plan,
        l,
        owner: "s",
        members: groups
            .iter()
            .flat_map(|g| g.group.fields())
            .map(str::to_string)
            .collect(),
    }
}

/// The bare Go type of each member of a group of `plan`, whose value's
/// struct is `struct_name`, by its id.
fn member_types(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
    l: &LangCtx,
    struct_name: &str,
) -> BTreeMap<String, String> {
    plan.groups()
        .iter()
        .flat_map(|g| g.fields())
        .map(|id| {
            let field = plan
                .model
                .fields
                .iter()
                .find(|f| f.id == id)
                .expect("a member of a group is a field of the codec");
            let held = held_value(
                imports,
                l,
                struct_name,
                field,
                &plan.value(field),
                Presence::Always,
            );
            (id.to_string(), held.bare)
        })
        .collect()
}

/// The statements, one tab in, that bind off the value each member of a
/// group `needed` names ([`bind_group`]), and read each group `whole` names.
fn bind_needed(
    l: &LangCtx,
    groups: &[GoGroup<'_>],
    member_types: &BTreeMap<String, String>,
    needed: &dyn Fn(&str) -> bool,
    whole: &BTreeSet<usize>,
) -> String {
    let mut out = String::new();
    for (at, g) in groups.iter().enumerate() {
        if whole.contains(&at) || g.group.fields().into_iter().any(needed) {
            out.push_str(&indented(&bind_group(l, g, member_types, "s", needed)));
        }
    }
    out
}

/// The number `measure` gives of the content `held` holds, an `int`, and the
/// statements, at `indent`, that count it first: the bytes of a byte string
/// or a text, less or more its `sce:length-arith`, or the bytes the body's own
/// encode writes over a sink that keeps none, handed each input it takes off
/// the carriers `carrier` reads. A body whose encode refuses has no frame; the
/// count is then of what it wrote before it refused.
fn held_measure(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
    carrier: &dyn Fn(&CodecField) -> String,
    measure: &Measure<'_>,
    held: &str,
    indent: &str,
) -> (String, String) {
    match &measure.of {
        Measured::Bytes { offset, .. } => (
            String::new(),
            format!("{held}.Len(){}", length_arith(offset)),
        ),
        // The entries of the list.
        Measured::Entries { .. } => (String::new(), format!("{held}.Len()")),
        Measured::Encoded { codec, .. } => (
            format!(
                "{indent}var _k codec.CountingSink\n\
                 {indent}_ = {}\n",
                body_encode(plan, imports, codec, held, "&_k", carrier, None)
            ),
            "_k.Position()".to_string(),
        ),
        _ => unreachable!("`unlowered` prints a length or a count of content only"),
    }
}

/// The call that writes `source`, a source of the body of the codec `alias`
/// holds, into `sink` by that codec's `EncodeSource`, handed each flag input
/// it takes and does not say: a flag of a carrier, read by `carrier`, or an
/// input of this codec's own, which the encode is handed. Where `overrides`
/// sets flags of the body in place of the ones it presents (the flag a chain
/// ends its entries on), by that codec's `EncodeSourceOverriding`.
fn body_encode(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
    alias: &str,
    source: &str,
    sink: &str,
    carrier: &dyn Fn(&CodecField) -> String,
    overrides: Option<&str>,
) -> String {
    let import = printed_import(imports, alias)
        .expect("`unlowered` prints a body of a codec the Go backend prints");
    let handed: String = plan
        .handed
        .get(alias)
        .into_iter()
        .flatten()
        .map(|h| match &h.from {
            Some(HandedFrom::Carrier {
                carrier: c,
                bit,
                width,
            }) => {
                let mask = if *width >= 64 {
                    u64::MAX
                } else {
                    (1u64 << width) - 1
                };
                format!(", byte(({} >> {bit}) & 0x{mask:X})", carrier(c))
            }
            Some(HandedFrom::Input { name }) => format!(", {}", input_name(name)),
            // What the entry before was, which the walk of a chain keeps as
            // it writes each entry; `unlowered` prints no other input bound to
            // nothing.
            None => format!(", {}", previous_local(&h.input)),
        })
        .collect();
    match overrides {
        None => format!(
            "{}.EncodeSource({source}, {sink}{handed})",
            import.namespace
        ),
        Some(overrides) => format!(
            "{}.EncodeSourceOverriding({source}, {sink}{handed}, {overrides})",
            import.namespace
        ),
    }
}

/// How the encode walks the list or chain `id` holds, where it holds one.
fn list_walk<'p>(plan: &'p EncodePlan<'_>, id: &str) -> Option<&'p ListWrite> {
    plan.program.iter().find_map(|write| match write {
        Write::Streamed {
            field,
            list: Some(walk),
            ..
        } if field.id == id => Some(walk),
        _ => None,
    })
}

/// The statements, unindented, that write each entry of the list or chain
/// `field`, read as `entries`, into `sink` by its entries' codec `alias`,
/// each handed what a body is ([`body_encode`]). A chain that ends on its
/// entries' flag sets the flag on every entry but the last, in place of the
/// one the entry presents, by the entries' codec's `Overrides`.
fn walk_entries(
    reads: &SourceReads<'_, '_>,
    imports: &[ImportContext],
    field: &CodecField,
    alias: &str,
    entries: &str,
    sink: &str,
    carrier: &dyn Fn(&CodecField) -> String,
) -> String {
    let (plan, l) = (reads.plan, reads.l);
    let name = l.codec_field_id(&field.id);
    let mut before = String::new();
    let mut each = String::new();
    let mut after = String::new();
    let mut overrides = None;
    let walk = list_walk(plan, &field.id);
    // Each input the chain hands an entry from the entry before
    // (`<sce:entry-flag-bind>`): clear for the first, and after each entry
    // whether it is of the kind the input waits for.
    for (input, kind) in walk.map(|w| w.previous.as_slice()).unwrap_or_default() {
        let local = previous_local(input);
        before.push_str(&format!("{local} := byte(0)\n"));
        after.push_str(&format!(
            "\t{local} = 0\n\
             \tif {} == 0x{kind:X} {{\n\
             \t\t{local} = 1\n\
             \t}}\n",
            entry_identifier(reads, &field.id, "_e")
        ));
    }
    if let Some((flag_carrier, flag)) = walk.and_then(|w| w.ends.as_ref()) {
        let import = printed_import(imports, alias)
            .expect("`unlowered` prints a chain of a codec the Go backend prints");
        let left = format!("_left{name}");
        before.push_str(&format!("{left} := {entries}.Len()\n"));
        each.push_str(&format!("\t{left}--\n"));
        overrides = Some(format!(
            "{}.{}Overrides{{{}: optional.Some({left} != 0)}}",
            import.namespace,
            import.type_name,
            override_member(l, flag_carrier, flag)
        ));
    }
    let call = body_encode(
        plan,
        imports,
        alias,
        "_e",
        sink,
        carrier,
        overrides.as_deref(),
    );
    let walked = format!(
        "{before}for _e := range {entries}.All() {{\n\
         {each}\
         \tif err := {call}; err != nil {{\n\
         \t\treturn err\n\
         \t}}\n\
         {after}\
         }}\n"
    );
    // The walk's own locals are its own, as the list is walked once to count
    // it and once to write it.
    if before.is_empty() {
        walked
    } else {
        format!("{{\n{}}}\n", indented(&walked))
    }
}

/// The local the walk of a chain keeps the input `input` of its entries in:
/// whether the entry it wrote last is of the kind the input waits for, 0
/// before the first.
fn previous_local(input: &str) -> String {
    format!("_prev{}", filters::to_pascal_case(input.to_string()))
}

/// The identifier of `entry`, a source of the entries of the chain `chain`,
/// as a `uint64`: the flag or the field the chain's `entry-id` names, read off
/// the entry by its method, less the flags it leaves out.
fn entry_identifier(reads: &SourceReads<'_, '_>, chain: &str, entry: &str) -> String {
    let l = reads.l;
    match reads
        .plan
        .entry_ids
        .get(chain)
        .expect("a chain that hands an entry what the one before was declares entry-id")
    {
        EntryId::Flag { carrier, bit, mask } => format!(
            "((uint64({entry}.{}()) >> {bit}) & 0x{mask:X})",
            l.codec_field_id(carrier)
        ),
        EntryId::Whole {
            field,
            mask: Some(mask),
        } => format!(
            "(uint64({entry}.{}()) & 0x{mask:X})",
            l.codec_field_id(field)
        ),
        EntryId::Whole { field, mask: None } => {
            format!("uint64({entry}.{}())", l.codec_field_id(field))
        }
    }
}

/// The field of a codec's `Overrides` that sets the flag `flag` of its
/// carrier `carrier`: the carrier's Go name and the flag's, which a chain
/// holding the codec spells alike.
fn override_member(l: &LangCtx, carrier: &str, flag: &str) -> String {
    format!(
        "{}{}",
        l.codec_field_id(carrier),
        filters::to_pascal_case(flag.to_string())
    )
}

/// The local of the sink that keeps nothing a body `body` is counted over,
/// with the statements, written into `out` once for each body, that encode
/// the body the source presents, where it presents one, by its codec `alias`
/// over that sink, handed the carriers `carrier` reads, and return its
/// refusal before the first byte of the frame.
fn count_body(
    imports: &[ImportContext],
    reads: &SourceReads<'_, '_>,
    body: &CodecField,
    alias: &str,
    carrier: &dyn Fn(&CodecField) -> String,
    counted: &mut BTreeSet<String>,
    out: &mut String,
) -> String {
    let plan = reads.plan;
    let k = format!("_k{}", reads.l.codec_field_id(&body.id));
    if !counted.insert(body.id.clone()) {
        return k;
    }
    // A body whose encode can refuse passes its refusal on, here.
    if plan.totality.refuses(RefusalKind::Child, &body.id) {
        plan.totality.require(RefusalKind::Child, &body.id);
    }
    // A list's entries, each in turn and each handed what the write hands
    // it; a body, whole.
    let sink = format!("&{k}");
    let check = if matches!(plan.value(body), Value::List { .. }) {
        walk_entries(
            reads,
            imports,
            body,
            alias,
            &reads.value(&body.id),
            &sink,
            carrier,
        )
    } else {
        let call = body_encode(
            plan,
            imports,
            alias,
            &reads.value(&body.id),
            &sink,
            carrier,
            None,
        );
        format!("if err := {call}; err != nil {{\n\treturn err\n}}\n")
    };
    out.push_str(&format!("\tvar {k} codec.CountingSink\n"));
    if reads.optional(&body.id) {
        out.push_str(&format!(
            "\tif {} {{\n{}\t}}\n",
            reads.present(&body.id),
            indented(&indented(&check))
        ));
    } else {
        out.push_str(&indented(&check));
    }
    k
}

/// The Go type a source presents the body of the codec `alias` holds as: that
/// codec's source interface, so any source of the body is one.
fn body_source(imports: &[ImportContext], alias: &str) -> (String, String) {
    let import = printed_import(imports, alias)
        .expect("`unlowered` prints a body of a codec the Go backend prints");
    (
        format!("{}.{}Source", import.namespace, import.type_name),
        format!("{}.{}", import.namespace, import.type_name),
    )
}

/// The `sce:length-arith` of a derived length as a Go term the count of bytes
/// is followed by: what the wire adds to it, or takes off.
fn length_arith(offset: &Offset) -> String {
    match offset {
        Offset::None => String::new(),
        Offset::More { by } => format!(" + {by}"),
        Offset::Less { by, .. } => format!(" - {by}"),
    }
}

/// The type a codec `struct_name` names for the bound of its byte string or
/// text `id`.
fn bound_type(struct_name: &str, id: &str) -> String {
    format!(
        "{struct_name}{}Bound",
        filters::to_pascal_case(id.to_string())
    )
}

/// The Go text the printer gives a codec's file.
pub(super) struct GoSource {
    /// Before the value's struct: the bound type of each byte string and text,
    /// and the interface a source of the codec satisfies.
    pub(super) before: String,
    /// After the value's struct: its constructor, the method that presents
    /// each element, and the encode over any source, `EncodeSource`.
    pub(super) after: String,
    /// For each element the value presents, by field id, the Go type its
    /// field is declared as: `codec.Bytes` or `codec.Text` of its bound for a
    /// byte string or a text, the `optional.Of` of what it holds where it may
    /// be absent.
    pub(super) held: BTreeMap<String, String>,
    /// For each byte string and text, by field id, the local the decode makes
    /// the bounded value of it in, for each length the value states, the
    /// optional value of the number the decode read, and for each body, the
    /// source of it the decoded value is: what the value's literal holds in
    /// place of what the decode read.
    pub(super) made: BTreeMap<String, String>,
    /// The statements the decode runs before the value's literal: each byte
    /// string and text it read made into its bounded value, refused as the
    /// runtime refuses it; empty where the value presents none.
    pub(super) decoded: String,
    /// For each arm of the variant the codec names, by the `variant_name` the
    /// template declares it by, the arm the decode makes of `_arm`, the value
    /// of the arm's codec it read, by its address.
    pub(super) arm_made: BTreeMap<String, String>,
    /// The catch-all arm the decode makes of `_arm` and `_tag`, the tag it read,
    /// where the variant has one: the decode reaches it only under a tag no
    /// arm names that the bits hold, so the tag is one of its type
    /// (docs/adr/0065 §6 R3) and is made without a check.
    pub(super) catch_all_made: Option<String>,
    /// Each gate group the value holds whole, in the order `Make{Codec}` takes
    /// them: the template declares its field and its decode's literal holds
    /// what the decode made of it. No member of a group is a field of the
    /// value's own.
    pub(super) groups: Vec<GoGroupField>,
}

/// The Go text of the codec `plan`, whose value's struct is `struct_name`; the
/// plan is one [`unlowered`] answers `None` for.
pub(super) fn render(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> GoSource {
    let mut before = String::new();
    let mut held = BTreeMap::new();
    let mut made = BTreeMap::new();
    let mut decoded = String::new();
    // Each element: its method, its field, and the Go type it is presented as.
    let mut elements: Vec<(String, String, String)> = Vec::new();
    // The value a method presents where the value holds no element of it: a
    // body the zero value holds none of presents its codec's zero value.
    let mut zero: BTreeMap<String, String> = BTreeMap::new();
    // The gate groups, each held whole where its first member is declared.
    let groups = go_groups(plan, struct_name);
    let mut group_fields: Vec<GoGroupField> = Vec::new();
    let mut held_groups: BTreeSet<usize> = BTreeSet::new();
    for field in &plan.model.fields {
        let id = &field.id;
        if let Some(element) = element_of(plan, id) {
            let method = l.codec_field_id(id);
            let value = held_value(
                imports,
                l,
                struct_name,
                field,
                &element.value,
                element.presence,
            );
            before.push_str(&value.declared);
            decoded.push_str(&value.decoded);
            if let Some(local) = value.made {
                made.insert(id.clone(), local);
            }
            if let Some(fallback) = value.zero {
                zero.insert(method.clone(), fallback);
            }
            held.insert(id.clone(), value.ty.clone());
            elements.push((method, member(id), value.ty));
            continue;
        }
        let Some(at) = groups.iter().position(|g| g.group.contains(id)) else {
            continue;
        };
        if !held_groups.insert(at) {
            continue;
        }
        let printed = group_declarations(plan, imports, l, struct_name, &groups[at]);
        before.push_str(&printed.declared);
        decoded.push_str(&printed.decoded);
        let g = &groups[at];
        if let Some(fallback) = printed.zero {
            zero.insert(g.method.clone(), fallback);
        }
        elements.push((g.method.clone(), g.member.clone(), g.held()));
        group_fields.push(GoGroupField {
            member: g.member.clone(),
            held: g.held(),
            made: printed.made,
        });
    }

    // The arm of the variant the value holds: a sum of the arm types, each a
    // source of its codec, presented and held after the elements.
    let mut arm_made = BTreeMap::new();
    let mut catch_all_made = None;
    if !plan.arms.is_empty() {
        let declared = variant_declarations(plan, imports, l, struct_name);
        before.push_str(&declared.text);
        arm_made = declared.made;
        catch_all_made = declared.catch_all;
        let zero_arm = plan
            .arms
            .iter()
            .find(|arm| arm.by_default)
            .map(|arm| format!("{}{{}}", plan_arm_type(struct_name, arm)))
            .expect("one arm of a variant is the one a value made by default holds");
        zero.insert(VARIANT_METHOD.to_string(), zero_arm);
        elements.push((
            VARIANT_METHOD.to_string(),
            VARIANT_MEMBER.to_string(),
            variant_type(struct_name),
        ));
    }

    before.push_str(&format!(
        "// {struct_name}Source is what the encode of {struct_name} reads: one method\n\
         // for each element a value presents (docs/adr/0081 §2). {struct_name} is one,\n\
         // and so is any value with these methods. The encode derives every other\n\
         // element of the frame.\n\
         type {struct_name}Source interface {{\n"
    ));
    for (method, _, ty) in &elements {
        before.push_str(&format!("\t{method}() {ty}\n"));
    }
    before.push('}');

    let parameters: Vec<String> = elements
        .iter()
        .map(|(_, field, ty)| format!("{field} {ty}"))
        .collect();
    let fields: Vec<String> = elements
        .iter()
        .map(|(_, field, _)| format!("{field}: {field}"))
        .collect();
    let mut after = format!(
        "// Make{struct_name} returns the {struct_name} that presents each element it\n\
         // is handed, in declaration order (docs/adr/0081 §2). Its fields are\n\
         // unexported: outside this package a value is made here, by\n\
         // Decode{struct_name}, or as its zero value, and is read through its methods.\n\
         func Make{struct_name}({}) {struct_name} {{\n\
         \treturn {struct_name}{{{}}}\n\
         }}\n",
        parameters.join(", "),
        fields.join(", ")
    );
    for (method, field, ty) in &elements {
        let held_none = match zero.get(method) {
            Some(zero) => format!(
                "\tif s.{field} == nil {{\n\
                 \t\treturn {zero}\n\
                 \t}}\n"
            ),
            None => String::new(),
        };
        let doc = if !plan.arms.is_empty() && method == VARIANT_METHOD && field == VARIANT_MEMBER {
            format!(
                "// {method} returns the arm of the variant the value holds; a value that\n\
                 // was handed none, as the zero value, holds {}.",
                zero[method]
            )
        } else {
            format!("// {method} returns the element the value presents as {method}.")
        };
        after.push_str(&format!(
            "\n{doc}\n\
             func (s *{struct_name}) {method}() {ty} {{\n\
             {held_none}\
             \treturn s.{field}\n\
             }}\n"
        ));
    }
    // The tag of the arm the value holds, where the caller hands the decode
    // the tag (a variant with no tag of its own): the tag the decode takes
    // for a value of the arm.
    if plan
        .model
        .variant
        .as_ref()
        .is_some_and(|variant| variant.tag_field.is_none())
    {
        after.push_str(&caller_tag(plan, struct_name));
    }
    after.push_str(&wire_methods(plan, l, imports, struct_name));
    after.push('\n');
    after.push_str(&encode_source(plan, l, imports, struct_name));

    GoSource {
        before,
        after,
        held,
        made,
        decoded: decoded.trim_end_matches('\n').to_string(),
        arm_made,
        catch_all_made,
        groups: group_fields,
    }
}

/// How a printed Go value holds one element, or one member of a group.
struct HeldValue {
    /// The Go type it is presented as: the optional type of what it holds
    /// where it may be absent.
    ty: String,
    /// The Go type of what it holds where it is there.
    bare: String,
    /// What the codec declares for it before the value: the bound type of a
    /// byte string, a text or a list.
    declared: String,
    /// The statements the decode runs before the value's literal to make it.
    decoded: String,
    /// What the value's literal holds in place of what the decode read, an
    /// optional where the decode read an optional; `None` where it holds the
    /// read itself.
    made: Option<String>,
    /// What its method presents where the value holds none: the zero value of
    /// a body's codec.
    zero: Option<String>,
}

/// How a printed Go value of the codec whose struct is `struct_name` holds
/// `field`, which holds `value` and whose presence is `presence`.
///
/// A body is presented as its codec's source interface, and held as one, so
/// a value is made of any source of the body; the decode holds the value it
/// read in it (docs/adr/0081 §2). A list or a chain is presented as the
/// runtime's `codec.List` of its entries' source interface, held to the
/// bound type the codec names for it (docs/adr/0065 §6 R2) and read through
/// an `iter.Seq`; the decode read the entries as values of their codec, and
/// the list holds their addresses, made within the bound or refused as the
/// runtime refuses it. A byte string or a text is the runtime's `codec.Bytes`
/// or `codec.Text` of its bound type (docs/adr/0077 §3), made of the slice or
/// the string the decode read, a slice made a string for a text.
fn held_value(
    imports: &[ImportContext],
    l: &LangCtx,
    struct_name: &str,
    field: &CodecField,
    value: &Value<'_>,
    presence: Presence,
) -> HeldValue {
    let id = &field.id;
    let method = l.codec_field_id(id);
    let optional = presence.optional();
    let wrap = |bare: &str| {
        if optional {
            format!("optional.Of[{bare}]")
        } else {
            bare.to_string()
        }
    };
    let made_in = |local: &str, make: &str| {
        format!(
            "\t{local}, err := {make}\n\
             \tif err != nil {{\n\
             \t\treturn nil, err\n\
             \t}}\n"
        )
    };
    match value {
        Value::Body { codec } => {
            let (source, held) = body_source(imports, codec);
            let (made, zero) = if optional {
                (
                    format!("optional.Convert({method}, func(b {held}) {source} {{ return &b }})"),
                    None,
                )
            } else {
                (format!("&{method}"), Some(format!("&{held}{{}}")))
            };
            HeldValue {
                ty: wrap(&source),
                bare: source,
                declared: String::new(),
                decoded: String::new(),
                made: Some(made),
                zero,
            }
        }
        Value::List { codec, bound, .. } => {
            let (source, held) = body_source(imports, codec);
            let bound_name = bound_type(struct_name, id);
            let (held_as, most) = match bound.most {
                Some(most) => (
                    format!("{} to {most} entries", bound.least),
                    most.to_string(),
                ),
                None => (
                    format!(
                        "{} or more entries, the protocol stating no most",
                        bound.least
                    ),
                    "codec.Unbounded".to_string(),
                ),
            };
            let declared = format!(
                "// {bound_name} is the bound of the {id} element of {struct_name}: {held_as}\n\
                 // (docs/adr/0065 §6 R2). A List of it is made only within it.\n\
                 type {bound_name} struct{{}}\n\
                 \n\
                 // Least is the fewest entries the element holds.\n\
                 func ({bound_name}) Least() int {{ return {least} }}\n\
                 \n\
                 // Most is the most entries the element holds.\n\
                 func ({bound_name}) Most() int {{ return {most} }}\n\n",
                least = bound.least
            );
            let list = format!("codec.List[{bound_name}, {source}]");
            let address = format!("func(e *{held}) {source} {{ return e }}");
            let make = if optional {
                format!(
                    "codec.MakeOptionalList[{bound_name}](codec.OptionalSourcesOf({method}, \
                     {address}))"
                )
            } else {
                format!("codec.MakeList[{bound_name}](codec.SourcesOf({method}, {address}))")
            };
            let local = format!("_l{method}");
            HeldValue {
                ty: wrap(&list),
                bare: list,
                declared,
                decoded: made_in(&local, &make),
                made: Some(local),
                zero: None,
            }
        }
        Value::Bytes(bound) | Value::Text(bound) => {
            let text = matches!(value, Value::Text(_));
            let (kind, unit) = if text {
                ("Text", "UTF-8 bytes")
            } else {
                ("Bytes", "bytes")
            };
            let bound_name = bound_type(struct_name, id);
            let (held_as, most) = match bound.most {
                Some(most) => (
                    format!("{} to {most} {unit}", bound.least),
                    most.to_string(),
                ),
                None => (
                    format!(
                        "{} or more {unit}, the protocol stating no most",
                        bound.least
                    ),
                    "codec.Unbounded".to_string(),
                ),
            };
            let declared = format!(
                "// {bound_name} is the bound of the {id} element of {struct_name}: {held_as}\n\
                 // (docs/adr/0077 §3). A {kind} of it is made only within it.\n\
                 type {bound_name} struct{{}}\n\
                 \n\
                 // Least is the fewest {unit} the element holds.\n\
                 func ({bound_name}) Least() int {{ return {least} }}\n\
                 \n\
                 // Most is the most {unit} the element holds.\n\
                 func ({bound_name}) Most() int {{ return {most} }}\n\n",
                least = bound.least
            );
            let bounded = format!("codec.{kind}[{bound_name}]");
            let make = if optional {
                format!("codec.MakeOptional{kind}[{bound_name}]({method})")
            } else if text {
                format!("codec.Make{kind}[{bound_name}](string({method}))")
            } else {
                format!("codec.Make{kind}[{bound_name}]({method})")
            };
            let local = format!("_b{method}");
            HeldValue {
                ty: wrap(&bounded),
                bare: bounded,
                declared,
                decoded: made_in(&local, &make),
                made: Some(local),
                zero: None,
            }
        }
        Value::Scalar(_) => {
            let scalar = l.type_name(&field.sce_type).into_owned();
            // A length the value states while its content is absent is on the
            // wire whether or not the content is there: the decode read it
            // as a number, which the value states.
            let made = (presence == Presence::Stated).then(|| format!("optional.Some({method})"));
            HeldValue {
                ty: wrap(&scalar),
                bare: scalar,
                declared: String::new(),
                decoded: String::new(),
                made,
                zero: None,
            }
        }
    }
}

/// A gate group of a printed Go codec as the printer spells it
/// (docs/adr/0065 §6 R4b, R4c).
struct GoGroup<'p> {
    /// The group.
    group: &'p GateGroup,
    /// The type the value holds it as: the interface its arm types satisfy
    /// where it has several arms, the struct of its one arm where it has one,
    /// named as every printer names it (`rust_source::group_type`).
    ty: String,
    /// The method that presents it, the source's and the value's.
    method: String,
    /// The field the value holds it in.
    member: String,
}

impl GoGroup<'_> {
    /// The Go type the value presents it as: the optional type of its type
    /// where the gates allow no arm.
    fn held(&self) -> String {
        if self.group.absent.is_some() {
            format!("optional.Of[{}]", self.ty)
        } else {
            self.ty.clone()
        }
    }

    /// The type of `arm`: a struct of its own named after the gates it holds
    /// (`rust_source::group_arm_name`) where the group has several arms, and
    /// the group's own type where it has one.
    fn arm_type(&self, arm: &GroupArm) -> String {
        if self.group.is_sum() {
            format!(
                "{}{}",
                self.ty,
                super::rust_source::group_arm_name(self.group, arm)
            )
        } else {
            self.ty.clone()
        }
    }

    /// The local the encode reads the group off its source into.
    fn read_local(&self) -> String {
        format!("_r{}", self.method)
    }
}

/// The gate groups of `plan`, whose value's struct is `struct_name`, in the
/// order of the plan.
fn go_groups<'p>(plan: &'p EncodePlan<'_>, struct_name: &str) -> Vec<GoGroup<'p>> {
    plan.groups()
        .iter()
        .map(|group| {
            let accessor = super::rust_source::group_accessor(group);
            GoGroup {
                group,
                ty: super::rust_source::group_type(struct_name, group),
                method: filters::to_pascal_case(accessor.clone()),
                member: member(&accessor),
            }
        })
        .collect()
}

/// The method and the field an arm holds the two lists `first` and `second`
/// of one count by (docs/adr/0065 §6 R5), named as every printer names it
/// (`rust_source::aligned_name`).
fn aligned_names(first: &str, second: &str) -> (String, String) {
    let name = super::rust_source::aligned_name(first, second);
    (filters::to_pascal_case(name.clone()), member(&name))
}

/// The local the encode binds the member `id` of a group to: an optional of
/// what it holds, there where the arm the value holds holds it.
fn member_local(l: &LangCtx, id: &str) -> String {
    format!("_m{}", l.codec_field_id(id))
}

/// A gate group as the template holds it in the value: the field, its type,
/// and what the decode's literal holds in it.
#[derive(serde::Serialize)]
pub(super) struct GoGroupField {
    /// The field.
    pub(super) member: String,
    /// Its Go type.
    pub(super) held: String,
    /// The local the decode made it in.
    pub(super) made: String,
}

/// What [`group_declarations`] prints of a group.
struct GroupDeclarations {
    /// Before the value's struct: the bound types of its members, and its
    /// types.
    declared: String,
    /// The statements the decode makes the group with, from the members it
    /// read, into the local `made` names.
    decoded: String,
    /// The local the decode makes the group in.
    made: String,
    /// What the value's method presents where it holds no group that must
    /// hold an arm: the first arm, made by default.
    zero: Option<String>,
}

/// The types of the group `g` of `plan`, whose value's struct is
/// `struct_name`: for each arm a struct that holds the members present while
/// the gates hold its values, each behind a method, made by its constructor;
/// where the group has several arms, the interface only they satisfy. Two
/// lists of one count are one member of the arm that holds both, the
/// runtime's `codec.Aligned` of them (docs/adr/0065 §6 R5). The decode makes
/// the arm the gates it read select, of the members it read, or none where
/// they select none.
fn group_declarations(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
    l: &LangCtx,
    struct_name: &str,
    g: &GoGroup<'_>,
) -> GroupDeclarations {
    let group = g.group;
    let field_of = |id: &str| {
        plan.model
            .fields
            .iter()
            .find(|f| f.id == id)
            .expect("a member of a group is a field of the codec")
    };
    // How each member is held, as the decode read it: the optional type of
    // what it holds where a gate of its own decides it.
    let mut members: BTreeMap<&str, HeldValue> = BTreeMap::new();
    let mut declared = String::new();
    let mut decoded = String::new();
    for id in group.fields() {
        let field = field_of(id);
        let presence = if field.present_if.is_some() {
            Presence::Gated
        } else {
            Presence::Always
        };
        let value = held_value(imports, l, struct_name, field, &plan.value(field), presence);
        declared.push_str(&value.declared);
        decoded.push_str(&value.decoded);
        members.insert(id, value);
    }
    // What the decode read of a member, as it is there.
    let read = |id: &str| -> String {
        let field = field_of(id);
        let local = members[id]
            .made
            .clone()
            .unwrap_or_else(|| l.codec_field_id(id));
        if field.present_if.is_some() {
            format!("{local}.Proven()")
        } else {
            local
        }
    };
    let when = |arm: &GroupArm| -> String {
        group
            .gates
            .iter()
            .zip(&arm.values)
            .map(|(gate, value)| {
                if *value {
                    format!("`{gate}` holds")
                } else {
                    format!("`{gate}` does not hold")
                }
            })
            .collect::<Vec<_>>()
            .join(" and ")
    };
    let marker = format!("is{}", g.ty);
    let local = format!("_g{}", g.method);
    let mut branches: Vec<(String, String)> = Vec::new();
    for arm in &group.arms {
        let ty = g.arm_type(arm);
        let mut fields = String::new();
        let mut parameters = Vec::new();
        let mut values = Vec::new();
        let mut methods = String::new();
        let mut arguments = Vec::new();
        let mut pairs = String::new();
        for id in &arm.fields {
            if group.is_second_in(arm, id) {
                continue;
            }
            let (method, name, member_ty, argument) = if let Some(second) = group.second_of(arm, id)
            {
                let (method, name) = aligned_names(id, second);
                let ty = format!(
                    "codec.Aligned[{}, {}]",
                    members[id.as_str()].bare,
                    members[second].bare
                );
                let pair = format!("_a{method}");
                // The decode read both lists by the one count, so they hold
                // as many entries as each other; it hands on the kind the
                // runtime answers, as it does for a byte string or a list it
                // makes (docs/adr/0099, amendment of 2026-11-16).
                pairs.push_str(&format!(
                    "\t{pair}, err := codec.MakeAligned({}, {})\n\
                     \tif err != nil {{\n\
                     \t\treturn nil, err\n\
                     \t}}\n",
                    read(id.as_str()),
                    read(second)
                ));
                (method, name, ty, pair)
            } else {
                (
                    l.codec_field_id(id),
                    member(id),
                    members[id.as_str()].bare.clone(),
                    read(id.as_str()),
                )
            };
            let held_none = match &members[id.as_str()].zero {
                Some(fallback) if group.second_of(arm, id).is_none() => format!(
                    "\tif a.{name} == nil {{\n\
                     \t\treturn {fallback}\n\
                     \t}}\n"
                ),
                _ => String::new(),
            };
            fields.push_str(&format!("\t{name} {member_ty}\n"));
            parameters.push(format!("{name} {member_ty}"));
            values.push(format!("{name}: {name}"));
            methods.push_str(&format!(
                "// {method} returns the member the arm presents as {method}.\n\
                 func (a {ty}) {method}() {member_ty} {{\n\
                 {held_none}\
                 \treturn a.{name}\n\
                 }}\n\n"
            ));
            arguments.push(argument);
        }
        declared.push_str(&format!(
            "// {ty} is the {} group of {struct_name} where {} (docs/adr/0065 §6 R4): the\n\
             // members present then.\n\
             type {ty} struct {{\n{fields}}}\n\n\
             // Make{ty} returns the arm that holds each member it is handed.\n\
             func Make{ty}({}) {ty} {{\n\
             \treturn {ty}{{{}}}\n\
             }}\n\n\
             {methods}",
            g.method,
            when(arm),
            parameters.join(", "),
            values.join(", ")
        ));
        if group.is_sum() {
            declared.push_str(&format!("func ({ty}) {marker}() {{}}\n\n"));
        }
        let made = format!("Make{ty}({})", arguments.join(", "));
        // The optional is of the group's type, which Go does not infer from an
        // arm of a sum: `optional.Some` is handed it by name.
        let made = if group.absent.is_some() {
            format!("optional.Some[{}]({made})", g.ty)
        } else {
            made
        };
        branches.push((
            decoded_test(plan, l, group, &arm.values),
            format!("{pairs}\t{local} = {made}\n"),
        ));
    }
    if group.is_sum() {
        let names: Vec<String> = group.arms.iter().map(|arm| g.arm_type(arm)).collect();
        declared.push_str(&format!(
            "// {} is the {} group of {struct_name}: the arm the value holds, each the\n\
             // members present while the gates hold its values (docs/adr/0065 §6 R4),\n\
             // one of {}, which only this package declares.\n\
             type {} interface {{\n\
             \t{marker}()\n\
             }}\n\n",
            g.ty,
            g.method,
            names.join(", "),
            g.ty
        ));
    }
    // The arm the gates the decode read select, in turn; where the gates may
    // select none, the value holds no arm, and the last arm is otherwise the
    // one they select.
    decoded.push_str(&format!("\tvar {local} {}\n", g.held()));
    let last = branches.len() - 1;
    for (i, (test, made)) in branches.iter().enumerate() {
        let keyword = if i == 0 { "\tif" } else { " else if" };
        if i == last && group.absent.is_none() && i > 0 {
            decoded.push_str(&format!(" else {{\n{}\t}}", indented(made)));
        } else if i == last && group.absent.is_none() {
            decoded.push_str(made);
        } else {
            decoded.push_str(&format!("{keyword} {test} {{\n{}\t}}", indented(made)));
        }
    }
    if !decoded.ends_with('\n') {
        decoded.push('\n');
    }
    let zero = (group.absent.is_none() && group.is_sum())
        .then(|| format!("{}{{}}", g.arm_type(&group.arms[0])));
    GroupDeclarations {
        declared,
        decoded,
        made: local,
        zero,
    }
}

/// Whether the gates of `group`, as the decode of the codec `plan` read them,
/// hold `values`: each flag tested on the local its carrier was read into.
fn decoded_test(plan: &EncodePlan<'_>, l: &LangCtx, group: &GateGroup, values: &[bool]) -> String {
    group
        .gates
        .iter()
        .zip(values)
        .map(|(gate, value)| {
            let compare = if *value { "!=" } else { "==" };
            match gate {
                GateVar::Flag { carrier, flag } => format!(
                    "(({} >> {}) & 1) {compare} 0",
                    l.codec_field_id(carrier),
                    flag_def(plan, carrier, flag).bit
                ),
                GateVar::Input(_) | GateVar::Member { .. } => {
                    unreachable!("`unlowered` prints a group of the codec's own flags only")
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" && ")
}

/// The statements, unindented, that bind each member of the group `g` of
/// `plan` the encode reads, read off `owner` (`src` or the value), to its
/// local ([`member_local`]): what the arm the value holds holds of it, absent
/// where that arm holds none or the value holds no arm. `needed` names the
/// members bound, each of which the statements after read. The group is read
/// into [`GoGroup::read_local`] first.
fn bind_group(
    l: &LangCtx,
    g: &GoGroup<'_>,
    member_types: &BTreeMap<String, String>,
    owner: &str,
    needed: &dyn Fn(&str) -> bool,
) -> String {
    let group = g.group;
    let read = g.read_local();
    let mut out = format!("{read} := {owner}.{}()\n", g.method);
    for id in group.fields() {
        if needed(id) {
            out.push_str(&format!(
                "var {} optional.Of[{}]\n",
                member_local(l, id),
                member_types[id]
            ));
        }
    }
    // What each arm holds of the members read.
    let assignments = |arm: &GroupArm| -> String {
        let mut out = String::new();
        for id in &arm.fields {
            if !needed(id) {
                continue;
            }
            let value = if let Some(second) = group.second_of(arm, id) {
                format!("_g.{}().First()", aligned_names(id, second).0)
            } else if group.is_second_in(arm, id) {
                let (first, _) = group
                    .aligned
                    .as_ref()
                    .expect("the second of two lists of one count is aligned");
                format!("_g.{}().Second()", aligned_names(first, id).0)
            } else {
                format!("_g.{}()", l.codec_field_id(id))
            };
            out.push_str(&format!(
                "{} = optional.Some({value})\n",
                member_local(l, id)
            ));
        }
        out
    };
    let held = if group.absent.is_some() {
        "_h"
    } else {
        read.as_str()
    };
    let body = if group.is_sum() {
        let mut cases = String::new();
        for arm in &group.arms {
            let set = assignments(arm);
            if !set.is_empty() {
                cases.push_str(&format!("case {}:\n{}", g.arm_type(arm), indented(&set)));
            }
        }
        if cases.is_empty() {
            String::new()
        } else {
            format!("switch _g := {held}.(type) {{\n{cases}}}\n")
        }
    } else {
        let set = assignments(&group.arms[0]);
        if set.is_empty() {
            String::new()
        } else {
            format!("_g := {held}\n{set}")
        }
    };
    if body.is_empty() {
        out
    } else if group.absent.is_some() {
        format!(
            "{out}if _h, ok := {read}.Get(); ok {{\n{}}}\n",
            indented(&body)
        )
    } else if group.is_sum() {
        format!("{out}{body}")
    } else {
        format!("{out}{{\n{}}}\n", indented(&body))
    }
}

/// The statements, unindented, that derive into `local`, a `uint64`, the
/// value the gate at `gate` among those of the group `g` has where the value
/// holds what [`GoGroup::read_local`] read: 1 where it holds an arm that sets
/// the gate, or no arm while the gates then set it.
fn group_gate_bit(g: &GoGroup<'_>, gate: usize, local: &str) -> String {
    let group = g.group;
    let read = g.read_local();
    let mut out = format!("var {local} uint64\n");
    let set: Vec<String> = group
        .arms
        .iter()
        .filter(|arm| arm.values[gate])
        .map(|arm| g.arm_type(arm))
        .collect();
    let when_absent = group.absent.as_ref().is_some_and(|absent| absent[gate]);
    let held = if group.absent.is_some() {
        "_h"
    } else {
        read.as_str()
    };
    let in_arm = if group.is_sum() {
        if set.is_empty() {
            String::new()
        } else {
            format!(
                "switch {held}.(type) {{\ncase {}:\n\t{local} = 1\n}}\n",
                set.join(", ")
            )
        }
    } else if set.is_empty() {
        String::new()
    } else {
        format!("{local} = 1\n")
    };
    match (&group.absent, group.is_sum()) {
        // No arm sets the gate: it is set only while the value holds none.
        (Some(_), _) if in_arm.is_empty() => {
            if when_absent {
                out.push_str(&format!("if !{read}.IsPresent() {{\n\t{local} = 1\n}}\n"));
            }
        }
        (Some(_), sum) => {
            let present = if sum {
                format!("if _h, ok := {read}.Get(); ok {{\n{}}}", indented(&in_arm))
            } else {
                format!("if {read}.IsPresent() {{\n{}}}", indented(&in_arm))
            };
            out.push_str(&present);
            if when_absent {
                out.push_str(&format!(" else {{\n\t{local} = 1\n}}"));
            }
            out.push('\n');
        }
        (None, _) => out.push_str(&in_arm),
    }
    out
}

/// The field a printed Go value holds the arm of its variant in.
const VARIANT_MEMBER: &str = crate::scxml_identifier::VARIANT_MEMBER;

/// What [`variant_declarations`] declares of a codec's variant.
struct VariantDeclarations {
    /// The Go text: the tag a catch-all holds, the variant's interface and
    /// each arm's type.
    text: String,
    /// [`GoSource::arm_made`].
    made: BTreeMap<String, String>,
    /// [`GoSource::catch_all_made`].
    catch_all: Option<String>,
}

/// The types of the variant of `plan`, whose value's struct is
/// `struct_name`: the interface the value holds the arm as, which only the
/// arm types satisfy, and for each arm a struct that holds a source of the
/// arm's codec behind its method, made by its constructor, the catch-all
/// beside its tag (docs/adr/0065 §6 R3). An arm that holds no source, as the
/// zero arm, presents its codec's value made by default.
fn variant_declarations(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
    l: &LangCtx,
    struct_name: &str,
) -> VariantDeclarations {
    let variant = variant_type(struct_name);
    let marker = format!("is{variant}");
    let names: Vec<String> = plan
        .arms
        .iter()
        .map(|arm| plan_arm_type(struct_name, arm))
        .collect();
    let mut text = String::new();
    // The tag a catch-all holds, within the tags no arm names.
    let unknown = crate::forge::codec_totality::unknown_tag(plan.model);
    let tag_type = super::rust_source::unknown_tag_type(struct_name);
    if let Some(unknown) = &unknown {
        text.push_str(&unknown_tag_declaration(struct_name, &tag_type, unknown, l));
    }
    text.push_str(&format!(
        "// {variant} is the arm of the variant a {struct_name} holds (docs/adr/0081 §2):\n\
         // one of {}, which only this package declares.\n\
         type {variant} interface {{\n\
         \t{marker}()\n\
         }}\n\n",
        names.join(", ")
    ));
    let mut made = BTreeMap::new();
    let mut catch_all = None;
    for (arm, ty) in plan.arms.iter().zip(&names) {
        let import = printed_import(imports, arm.codec)
            .expect("`unlowered` prints a variant of printed arms only");
        let (source, _) = body_source(imports, arm.codec);
        // What an arm made by default holds: the arm's codec's value made by
        // its constructor where its codec declares one, so the flags the
        // codec gives a value made by default hold, and its zero value
        // otherwise.
        let fallback = if import.codec_emits_default_ctor {
            format!("{}.New{}()", import.namespace, import.type_name)
        } else {
            format!("&{}.{}{{}}", import.namespace, import.type_name)
        };
        let chosen = match arm.tag {
            Some(tag) => format!("chosen by the tag 0x{tag:X}"),
            None => "chosen by every tag no arm names".to_string(),
        };
        let (fields, parameters, values) = match arm.tag {
            Some(_) => (
                format!("\tbody {source}\n"),
                format!("body {source}"),
                "body: body".to_string(),
            ),
            None => (
                format!("\ttag  {tag_type}\n\tbody {source}\n"),
                format!("tag {tag_type}, body {source}"),
                "tag: tag, body: body".to_string(),
            ),
        };
        text.push_str(&format!(
            "// {ty} is the arm of {variant} whose body is a {}, {chosen}.\n\
             type {ty} struct {{\n{fields}}}\n\n\
             // Make{ty} returns the arm that holds what it is handed.\n\
             func Make{ty}({parameters}) {ty} {{\n\
             \treturn {ty}{{{values}}}\n\
             }}\n\n\
             // Body returns the body the arm holds; an arm that holds none, as the zero\n\
             // arm, holds the {} its codec makes by default.\n\
             func (a {ty}) Body() {source} {{\n\
             \tif a.body == nil {{\n\
             \t\treturn {fallback}\n\
             \t}}\n\
             \treturn a.body\n\
             }}\n\n",
            import.type_name, import.type_name
        ));
        match arm.tag {
            Some(_) => {
                // Keyed by the template's `variant_name` of the arm, which
                // the decode's case of the arm reads it by.
                made.insert(variant_arm_name(Some(arm.codec)), format!("Make{ty}(_arm)"));
            }
            None => {
                text.push_str(&format!(
                    "// Tag returns the tag the arm holds, which no arm names.\n\
                     func (a {ty}) Tag() {tag_type} {{\n\
                     \treturn a.tag\n\
                     }}\n\n"
                ));
                catch_all = Some(format!(
                    "Make{ty}({tag_type}{{tag: _tag, set: true}}, _arm)"
                ));
            }
        }
        text.push_str(&format!("func ({ty}) {marker}() {{}}\n\n"));
    }
    VariantDeclarations {
        text,
        made,
        catch_all,
    }
}

/// The type `class` the catch-all arm of `struct_name`'s variant holds its
/// tag in (docs/adr/0065 §6 R3): a struct with unexported fields made only by
/// its constructor, which makes a value only of a tag the bits it is read
/// from hold and no arm names, so the catch-all a value holds is written and
/// read back as the catch-all, and the encode refuses nothing of it. Its
/// zero value holds the least such tag.
fn unknown_tag_declaration(
    struct_name: &str,
    class: &str,
    unknown: &crate::forge::codec_totality::UnknownTag,
    l: &LangCtx,
) -> String {
    let ty = l.type_name(&unknown.ty).into_owned();
    // The tags no value of the class holds: those the arms name, each run of
    // consecutive ones a range, and those past the bits.
    let mut named: Vec<String> = unknown
        .known_runs()
        .iter()
        .map(|&(first, last)| match (first, last) {
            (first, last) if first == last => format!("tag == 0x{first:X}"),
            (0, last) => format!("tag <= 0x{last:X}"),
            (first, last) => format!("tag >= 0x{first:X} && tag <= 0x{last:X}"),
        })
        .collect();
    if let Some(max) = unknown.max_below_type() {
        named.push(format!("tag > 0x{max:X}"));
    }
    let refused = if named.is_empty() {
        String::new()
    } else {
        format!(
            "\tif {} {{\n\
             \t\treturn {class}{{}}, false\n\
             \t}}\n",
            named.join(" || ")
        )
    };
    let fewest = (0..=unknown.max()).find(|tag| unknown.admits(*tag)).expect(
        "a variant whose every tag an arm names has no reachable catch-all \
             (codec/variant-arm-unreachable)",
    );
    format!(
        "// {class} is the tag the catch-all arm of {struct_name}'s variant holds: one\n\
         // the bits the tag is read from hold and no arm names, held to that where a\n\
         // value of it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all\n\
         // a value holds is written and read back as the catch-all. Its zero value\n\
         // holds 0x{fewest:X}, the least such tag.\n\
         type {class} struct {{\n\
         \ttag {ty}\n\
         \tset bool\n\
         }}\n\n\
         // Make{class} returns tag as the tag of the catch-all, and false where an\n\
         // arm names it or the bits the tag is read from do not hold it.\n\
         func Make{class}(tag {ty}) ({class}, bool) {{\n\
         {refused}\
         \treturn {class}{{tag: tag, set: true}}, true\n\
         }}\n\n\
         // Value returns the tag.\n\
         func (t {class}) Value() {ty} {{\n\
         \tif !t.set {{\n\
         \t\treturn 0x{fewest:X}\n\
         \t}}\n\
         \treturn t.tag\n\
         }}\n\n"
    )
}

/// The method of the value `struct_name` that answers the tag of the arm it
/// holds, for a variant whose caller hands the decode the tag: the tag the
/// arm declares, or the one the catch-all holds. It is the tag the decode
/// takes for a value of the arm.
fn caller_tag(plan: &EncodePlan<'_>, struct_name: &str) -> String {
    let mut cases = String::new();
    for arm in &plan.arms {
        let ty = plan_arm_type(struct_name, arm);
        cases.push_str(&match arm.tag {
            Some(tag) => format!("\tcase {ty}:\n\t\treturn 0x{tag:X}\n"),
            None => format!("\tcase {ty}:\n\t\treturn byte(_b.Tag().Value())\n"),
        });
    }
    let subject = if plan.arms.iter().any(|arm| arm.tag.is_none()) {
        format!("_b := s.{VARIANT_METHOD}().(type)")
    } else {
        format!("s.{VARIANT_METHOD}().(type)")
    };
    format!(
        "\n// Tag returns the tag of the arm this value holds: the one the arm declares,\n\
         // or, for the catch-all arm, the tag it holds. It is the `tag` that\n\
         // Decode{struct_name} takes for a value of this arm.\n\
         func (s *{struct_name}) Tag() byte {{\n\
         \tswitch {subject} {{\n\
         {cases}\
         \t}}\n\
         \tpanic(\"{struct_name}: the value holds no variant arm\")\n\
         }}\n"
    )
}

/// One write of `bytes`, the byte expressions of a write, into the sink.
fn write_bytes(bytes: &[String]) -> String {
    format!(
        "\tif err := w.WriteBytes([]byte{{{}}}); err != nil {{\n\t\treturn err\n\t}}\n",
        bytes.join(", ")
    )
}

/// One write of the slice `slice` into the sink.
fn write_slice(slice: &str) -> String {
    format!("\tif err := w.WriteBytes({slice}); err != nil {{\n\t\treturn err\n\t}}\n")
}

/// Each line of `body`, Go statements, one tab further in.
fn indented(body: &str) -> String {
    body.lines()
        .map(|line| {
            if line.is_empty() {
                "\n".to_string()
            } else {
                format!("\t{line}\n")
            }
        })
        .collect()
}

/// How the encode reads an element off its source.
struct SourceReads<'p, 'm> {
    /// The plan of the codec.
    plan: &'p EncodePlan<'m>,
    /// The Go spelling of the codec's names.
    l: &'p LangCtx,
    /// What reads the source: `src`, or the value.
    owner: &'p str,
    /// The members of the gate groups, each by the local the arm the value
    /// holds is read into ([`bind_group`]), an optional.
    members: BTreeSet<String>,
}

impl SourceReads<'_, '_> {
    /// The call of the method that presents the element `id`, or the local a
    /// member of a group is bound to, the optional type of what it holds.
    fn call(&self, id: &str) -> String {
        if self.members.contains(id) {
            member_local(self.l, id)
        } else {
            format!("{}.{}()", self.owner, self.l.codec_field_id(id))
        }
    }

    /// The element `id` as the encode reads it where it is there: the call,
    /// or what the optional type the call returns holds, read where a test
    /// has proven it present.
    fn value(&self, id: &str) -> String {
        if self.members.contains(id)
            || element_of(self.plan, id).is_some_and(|e| e.presence.optional())
        {
            format!("{}.Proven()", self.call(id))
        } else {
            self.call(id)
        }
    }

    /// Whether the element `id` may be absent where the encode reads it: an
    /// element a gate decides or the value states, or a member of a group.
    fn optional(&self, id: &str) -> bool {
        self.members.contains(id)
            || element_of(self.plan, id).is_some_and(|e| e.presence.optional())
    }

    /// The test that the element `id`, which may be absent, is there.
    fn present(&self, id: &str) -> String {
        format!("{}.IsPresent()", self.call(id))
    }
}

/// The Go name of the flag input `name`: the parameter the encode and the
/// decode are handed it by, as the template names it.
fn input_name(name: &str) -> String {
    filters::to_camel_case(name.to_string())
}

/// The local the encode holds the carrier `id` in as it writes it, its flags
/// the encode derives folded in.
fn carrier_local(l: &LangCtx, id: &str) -> String {
    format!("_c{}", l.codec_field_id(id))
}

/// The flag `flag` of the carrier `field` of `plan`.
fn flag_def<'m>(plan: &EncodePlan<'m>, field: &str, flag: &str) -> &'m FlagDef {
    plan.model
        .fields
        .iter()
        .find(|f| f.id == field)
        .and_then(|carrier| carrier.flags.iter().find(|d| d.name == flag))
        .expect("the parser found the flag a derivation names")
}

/// The bits of `def`, unshifted.
fn flag_width_mask(def: &FlagDef) -> u64 {
    if def.width >= 64 {
        u64::MAX
    } else {
        (1u64 << def.width) - 1
    }
}

/// The statement that returns the refusal of `kind` at `at`, the runtime's
/// sentinel of the refusal's name, which the totality analysis names
/// (`Totality::require`, docs/adr/0065 §2).
fn refusal(plan: &EncodePlan<'_>, kind: RefusalKind, at: &str) -> String {
    plan.totality.require(kind, at);
    let variant = kind
        .error_variant()
        .expect("a refusal the printer writes is one of the codec's own");
    format!("return codec.Err{variant}")
}

/// The encode of `struct_name` over any source of it, a function of the
/// codec's package, which is the codec's namespace as a C++ codec's namespace
/// is: each refusal the plan's totality names checked before the first byte,
/// each derived number made from what it measures, each derived flag folded
/// into its carrier, then the writes in wire order, each element read off the
/// source by its method. A function of the package and not a method of the
/// value, so a source need not be a value of the codec. A body is written by
/// its codec's own `EncodeSource`; one whose encode can refuse is first
/// encoded over a sink that keeps nothing, which returns its refusal before
/// the first byte and counts its frame for a length that measures it
/// (docs/adr/0058 §3).
fn encode_source(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> String {
    let numbers = derived_numbers(plan);
    let (stated, derived): (Vec<&String>, Vec<&String>) = numbers
        .iter()
        .partition(|id| element_of(plan, id).is_some());
    let named = |ids: &[&String]| {
        ids.iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut derives = String::new();
    if !derived.is_empty() {
        derives.push_str(&format!(
            " It derives {} from\n// what each measures, and never reads it off the source.",
            named(&derived)
        ));
    }
    if !stated.is_empty() {
        derives.push_str(&format!(
            " It derives {} where\n// what it measures is there, and reads what the source states \
             where none is.",
            named(&stated)
        ));
    }
    let tags = wire_tags(plan);
    if !tags.is_empty() {
        derives.push_str(&format!(
            " It derives {} from\n// the arm of the variant the source presents.",
            tags.join(", ")
        ));
    }
    // What the encode refuses: nothing, or what the plan's totality names,
    // each the runtime's sentinel of its name, before the first byte.
    let refuses = if plan.totality.is_total() {
        " It refuses no value: it returns only the\n// error of w".to_string()
    } else {
        let mut returned: Vec<String> = plan
            .totality
            .refusals()
            .iter()
            .filter_map(|r| r.kind.error_variant())
            .map(|variant| format!("codec.Err{variant}"))
            .collect();
        returned.sort();
        returned.dedup();
        // A body whose encode can refuse returns its codec's refusal.
        if plan
            .totality
            .refusals()
            .iter()
            .any(|r| r.kind == RefusalKind::Child)
        {
            returned.push("the refusal of a body it holds".to_string());
        }
        format!(
            " It returns {} for\n// a value no frame carries, before it writes a byte to w, and \
             otherwise only the\n// error of w",
            returned.join(", ")
        )
    };
    // The flag inputs the encode is handed, those the value does not say,
    // after the sink, as the decode is handed them after the cursor.
    let given: String = plan
        .given
        .iter()
        .map(|input| format!(", {} byte", input_name(&input.name)))
        .collect();
    let mut out = format!(
        "// EncodeSource writes any source of {struct_name} into w, each element read\n\
         // off src and written where the codec's encode plan lays it out\n\
         // (docs/adr/0081).{derives}{refuses}, codec.ErrBufferOverflow from a bounded sink\n\
         // that has no room left.\n\
         func EncodeSource(src {struct_name}Source, w codec.SceSink{given}) error {{\n"
    );
    // The one-bit flags a parent may set in place of the ones a value
    // presents (the flag a chain ends its entries on, which the chain derives
    // from where each entry stands): a struct of them, and an encode that
    // takes it, which `EncodeSource` calls setting none.
    if !plan.overridable.is_empty() {
        let mut members = String::new();
        for (carrier, def) in &plan.overridable {
            members.push_str(&format!(
                "\t// {} is the `{}` flag of `{}`, where a parent sets it.\n\t{} optional.Of[bool]\n",
                override_member(l, &carrier.id, &def.name),
                def.name,
                carrier.id,
                override_member(l, &carrier.id, &def.name)
            ));
        }
        let handed: String = plan
            .given
            .iter()
            .map(|input| format!(", {}", input_name(&input.name)))
            .collect();
        out = format!(
            "// {struct_name}Overrides holds the one-bit flags of {struct_name} a parent may\n\
             // set in place of the ones a value presents, such as the flag a chain whose\n\
             // entries are {struct_name} ends them on, which the chain derives from where\n\
             // each entry stands. A flag it leaves absent is the one the value presents.\n\
             type {struct_name}Overrides struct {{\n{members}}}\n\n\
             {out}\
             \treturn EncodeSourceOverriding(src, w{handed}, {struct_name}Overrides{{}})\n}}\n\n\
             // EncodeSourceOverriding is EncodeSource, each flag overrides sets written in\n\
             // place of the one src presents.\n\
             func EncodeSourceOverriding(src {struct_name}Source, w codec.SceSink{given}, \
             overrides {struct_name}Overrides) error {{\n"
        );
    }
    let groups = go_groups(plan, struct_name);
    let reads = SourceReads {
        plan,
        l,
        owner: "src",
        members: groups
            .iter()
            .flat_map(|g| g.group.fields())
            .map(str::to_string)
            .collect(),
    };
    // The arm of the variant the source presents, read once: the tag the
    // encode derives and the arm it writes are of the one arm.
    if !plan.arms.is_empty() {
        out.push_str(&format!("\t{VARIANT_LOCAL} := src.{VARIANT_METHOD}()\n"));
    }
    // Each group the source presents, read once, each member bound to a local
    // of its own from the arm it holds, so each statement that writes a
    // member is the one that writes an element a gate decides.
    let member_types = member_types(plan, imports, l, struct_name);
    for g in &groups {
        out.push_str(&indented(&bind_group(l, g, &member_types, "src", &|_| {
            true
        })));
    }
    // Each flag the encode derives, by its carrier: the statements that fold
    // it into the carrier's local in place of the bits the value presents.
    let mut folds: BTreeMap<String, Vec<String>> = BTreeMap::new();
    // The bodies counted over a sink that keeps nothing, each once.
    let mut counted: BTreeSet<String> = BTreeSet::new();
    // The numbers that count a body's frame, counted once every carrier is
    // folded, so a body is handed each flag as the encode writes it. Such a
    // number counts nothing else and goes into a field (`sce:length-from`
    // names a field declared before the body), so no fold waits on it.
    let mut encoded: Vec<(&ElementId, &Number<'_>)> = Vec::new();
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            (element, Derive::Number(number))
                if number
                    .measures
                    .iter()
                    .any(|measure| matches!(measure.of, Measured::Encoded { .. })) =>
            {
                encoded.push((element, number));
            }
            (element, Derive::Number(number)) => {
                let measured: Vec<MeasureText> = number
                    .measures
                    .iter()
                    .map(|measure| plain_measure(&reads, measure))
                    .collect();
                if let Some((carrier, statement)) =
                    derive_number(plan, l, &reads, element, &number.into, &measured, &mut out)
                {
                    folds.entry(carrier).or_default().push(statement);
                }
            }
            // Set exactly where the element it gates is there, or clear where
            // the element is there on the flag's clear state.
            (ElementId::Flag { field, flag }, Derive::Presence(presence)) => {
                let (gated, when_clear) =
                    presence.of.first().expect("a presence flag gates a field");
                let def = flag_def(plan, field, flag);
                let mask = flag_width_mask(def) << def.bit;
                let c = carrier_local(l, field);
                let test = if *when_clear {
                    format!("!{}", reads.present(&gated.id))
                } else {
                    reads.present(&gated.id)
                };
                folds.entry(field.clone()).or_default().push(format!(
                    "\t{c} &^= 0x{mask:X}\n\
                     \tif {test} {{\n\
                     \t\t{c} |= 0x{mask:X}\n\
                     \t}}\n"
                ));
            }
            // Set by the walk of the chain that holds the entry, not here.
            (ElementId::Flag { .. }, Derive::ChainContinuation) => {}
            // The tag of the arm the value holds: the value the document
            // gives a named arm, and the tag the catch-all holds; into its
            // field, or masked to the flag's bits, which leave the carrier's
            // other bits as the value presents them.
            (element, Derive::VariantTag) => {
                out.push_str(&indented(&arm_tag(plan, struct_name, VARIANT_LOCAL)));
                match element {
                    ElementId::Field(id) => {
                        let field = plan
                            .model
                            .fields
                            .iter()
                            .find(|f| &f.id == id)
                            .expect("a variant's tag is derived into a field of the codec");
                        let ty = l.type_name(&field.sce_type);
                        out.push_str(&format!(
                            "\t_d{} := {ty}({ARM_TAG_LOCAL})\n",
                            l.codec_field_id(id)
                        ));
                    }
                    ElementId::Flag { field, flag } => {
                        let c = carrier_local(l, field);
                        folds.entry(field.clone()).or_default().push(format!(
                            "\t{}",
                            fold_bits(plan, l, field, flag, ARM_TAG_LOCAL, &c)
                        ));
                    }
                    ElementId::Variant => unreachable!(
                        "a variant whose caller hands its tag derives none: the caller writes it"
                    ),
                }
            }
            // The tag of the arm each embedded body's variant holds, masked to
            // the flag's bits, the catch-all's the tag it holds; nothing where
            // a gate leaves the body out, and the bits the value presents where
            // no body is there to fill them.
            (ElementId::Flag { field, flag }, Derive::ParentTag(tag)) => {
                let local = format!("_p{}", derived_local(l, &derived.element));
                out.push_str(&indented(&parent_tag_bits(
                    plan, l, imports, tag, &local, "src",
                )));
                let c = carrier_local(l, field);
                folds
                    .entry(field.clone())
                    .or_default()
                    .push(indented(&fold_where_said(
                        parent_tag_said(plan, l, tag, "src"),
                        &fold_bits(plan, l, field, flag, &local, &c),
                    )));
            }
            // A flag of a disjunction that decides a group: what the arm the
            // value holds gives it, or what the gates hold while it holds none
            // (docs/adr/0065 §6 R4c).
            (ElementId::Flag { field, flag }, Derive::ArmFlag { group, gate }) => {
                let local = format!("_a{}", derived_local(l, &derived.element));
                out.push_str(&indented(&group_gate_bit(&groups[*group], *gate, &local)));
                let c = carrier_local(l, field);
                folds
                    .entry(field.clone())
                    .or_default()
                    .push(format!("\t{}", fold_bits(plan, l, field, flag, &local, &c)));
            }
            _ => unreachable!("`unlowered` prints no other derivation"),
        }
    }
    // A flag a parent sets in place of the one the value presents, folded
    // last: what the parent says is what the wire holds.
    for (carrier, def) in &plan.overridable {
        let mask = 1u64 << def.bit;
        let c = carrier_local(l, &carrier.id);
        folds.entry(carrier.id.clone()).or_default().push(format!(
            "\tif _o, ok := overrides.{}.Get(); ok {{\n\
             \t\t{c} &^= 0x{mask:X}\n\
             \t\tif _o {{\n\
             \t\t\t{c} |= 0x{mask:X}\n\
             \t\t}}\n\
             \t}}\n",
            override_member(l, &carrier.id, &def.name)
        ));
    }
    // Each carrier as the encode writes it: its flags the value presents, and
    // those the encode derives folded in, in place of the value's.
    for (carrier, carried) in &folds {
        out.push_str(&format!(
            "\t{} := {}\n",
            carrier_local(l, carrier),
            reads.call(carrier)
        ));
        for statement in carried {
            out.push_str(statement);
        }
    }
    let wholes = derived_fields(plan);
    let read = |f: &CodecField| -> String {
        if wholes.contains(&f.id) {
            format!("_d{}", l.codec_field_id(&f.id))
        } else if folds.contains_key(&f.id) {
            carrier_local(l, &f.id)
        } else {
            reads.value(&f.id)
        }
    };
    // How many bytes each body a number counts encodes to, over a sink that
    // keeps nothing, handed the carriers as the encode writes them.
    for (element, number) in encoded {
        let measured: Vec<MeasureText> = number
            .measures
            .iter()
            .map(|measure| {
                let Measured::Encoded { body, codec, .. } = &measure.of else {
                    unreachable!("a number of encoded lengths counts nothing else");
                };
                let k = count_body(imports, &reads, body, codec, &read, &mut counted, &mut out);
                MeasureText {
                    size: format!("{k}.Position()"),
                    present: measure.gated.then(|| reads.present(&body.id)),
                }
            })
            .collect();
        if derive_number(plan, l, &reads, element, &number.into, &measured, &mut out).is_some() {
            unreachable!("an encoded length goes into the field `sce:length-from` names");
        }
    }
    // A gate, tested on the carriers as the encode writes them and on the
    // inputs it is handed.
    let gate_test = |f: &CodecField| -> String {
        let gate = f
            .present_if
            .as_ref()
            .expect("a field written where its gate holds is gated");
        plan.clauses(gate)
            .iter()
            .map(|clause| match clause {
                Clause::Flag {
                    carrier,
                    bit,
                    negate,
                } => format!(
                    "(({} >> {bit}) & 1) {} 0",
                    read(carrier),
                    if *negate { "==" } else { "!=" }
                ),
                Clause::Input { name, negate } => format!(
                    "{} {} 0",
                    input_name(name),
                    if *negate { "==" } else { "!=" }
                ),
                Clause::Member { .. } => {
                    unreachable!("`unlowered` prints no gate on a chain's entries")
                }
            })
            .collect::<Vec<_>>()
            .join(" || ")
    };
    // Each gate the encode cannot derive the presence from holds the field it
    // gates, before the first byte (docs/adr/0065 §2).
    for field in &plan.gate_checks {
        out.push_str(&format!(
            "\tif ({}) != {} {{\n\t\t{}\n\t}}\n",
            gate_test(field),
            reads.present(&field.id),
            refusal(plan, RefusalKind::PresentIfMismatch, &field.id)
        ));
    }
    // A body whose encode can refuse, which no length counted, and each entry
    // of a list whose entries' encode can, is encoded over a sink that keeps
    // nothing before the first byte, so its refusal leaves w as it was.
    // A member of a group likewise, where the arm the value holds holds it.
    let held_fields = plan.elements.iter().map(|element| element.field).chain(
        groups
            .iter()
            .flat_map(|g| g.group.fields())
            .filter_map(|id| plan.model.fields.iter().find(|f| f.id == id)),
    );
    for field in held_fields {
        if let Value::Body { codec } | Value::List { codec, .. } = plan.value(field) {
            if plan.totality.refuses(RefusalKind::Child, &field.id) {
                count_body(imports, &reads, field, codec, &read, &mut counted, &mut out);
            }
        }
    }
    // The arm the value holds, where its codec's encode can refuse, likewise.
    let mut refusing = String::new();
    for arm in &plan.arms {
        if !plan.totality.refuses(RefusalKind::Child, arm.codec) {
            continue;
        }
        plan.totality.require(RefusalKind::Child, arm.codec);
        let call = body_encode(
            plan,
            imports,
            arm.codec,
            &format!("_b.{VARIANT_METHOD}()"),
            "&_kBody",
            &read,
            None,
        );
        refusing.push_str(&format!(
            "case {}:\n\
             \tvar _kBody codec.CountingSink\n\
             \tif err := {call}; err != nil {{\n\
             \t\treturn err\n\
             \t}}\n",
            plan_arm_type(struct_name, arm)
        ));
    }
    if !refusing.is_empty() {
        out.push_str(&indented(&format!(
            "switch _b := {VARIANT_LOCAL}.(type) {{\n{refusing}}}\n"
        )));
    }
    // A byte string's bytes or a text's, as the slice the sink writes.
    let slice = |f: &CodecField| -> String {
        if f.sce_type == SceType::String {
            format!("[]byte({}.String())", read(f))
        } else {
            format!("{}.Bytes()", read(f))
        }
    };
    let endian = plan.model.default_endian;
    for write in &plan.program {
        match write {
            Write::Whole(field) => {
                out.push_str(&write_bytes(&encode_whole_field_read_as(
                    field,
                    endian,
                    &read(field),
                    l,
                )));
            }
            Write::Byte { pieces, .. } => {
                out.push_str(&write_bytes(&[encode_byte_read_by(pieces, l, &read)]));
            }
            Write::Tail { field, pad } => {
                // A field the layout aligns follows zeros up to its offset.
                if *pad {
                    out.push_str(&format!(
                        "\tfor w.Position() < {} {{\n\
                         \t\tif err := w.WriteBytes([]byte{{0}}); err != nil {{\n\
                         \t\t\treturn err\n\
                         \t\t}}\n\
                         \t}}\n",
                        field.byte_offset
                    ));
                }
                out.push_str(&write_slice(&slice(field)));
            }
            Write::Streamed { field, guard, .. } => {
                let body = match &field.bit_size {
                    BitSize::Fixed { .. } => {
                        write_bytes(&encode_whole_field_read_as(field, endian, &read(field), l))
                    }
                    // The runtime's writer, as the template's encode calls it.
                    BitSize::Vle { width_bits } => {
                        let value = l.codec_carrier_expr(&field.sce_type, &read(field));
                        format!("{}\n", vle_encode_block(&value, *width_bits, l.lang))
                    }
                    BitSize::Tail | BitSize::LengthRef => write_slice(&slice(field)),
                    // By the body's codec's own encode, into the same sink.
                    BitSize::Embed => {
                        let Value::Body { codec } = plan.value(field) else {
                            unreachable!("an embedded field holds a body");
                        };
                        format!(
                            "\tif err := {}; err != nil {{\n\t\treturn err\n\t}}\n",
                            body_encode(plan, imports, codec, &read(field), "w", &read, None)
                        )
                    }
                    // Each entry in turn, by its codec's own encode.
                    BitSize::Repeat { .. } | BitSize::TlvChain { .. } => {
                        let Value::List { codec, .. } = plan.value(field) else {
                            unreachable!("a repeated field or a chain holds a list");
                        };
                        indented(&walk_entries(
                            &reads,
                            imports,
                            field,
                            codec,
                            &read(field),
                            "w",
                            &read,
                        ))
                    }
                };
                out.push_str(&match guard {
                    Guard::Always => body,
                    // Where the value holds it.
                    Guard::Present => format!(
                        "\tif {} {{\n{}\t}}\n",
                        reads.present(&field.id),
                        indented(&body)
                    ),
                    // Derived whole and gated: where its gate holds, as the
                    // carriers the encode writes say.
                    Guard::Gate => {
                        format!("\tif {} {{\n{}\t}}\n", gate_test(field), indented(&body))
                    }
                });
            }
            // The arm the value holds, written by its codec's encode over the
            // source the arm holds, handed what the arm's codec is handed. A
            // source that presents no arm of the codec's own breaks the
            // interface's contract, which only this package's arm types
            // satisfy.
            Write::Variant => {
                let mut cases = String::new();
                for arm in &plan.arms {
                    let call = body_encode(
                        plan,
                        imports,
                        arm.codec,
                        &format!("_b.{VARIANT_METHOD}()"),
                        "w",
                        &read,
                        None,
                    );
                    cases.push_str(&format!(
                        "case {}:\n\
                         \tif err := {call}; err != nil {{\n\
                         \t\treturn err\n\
                         \t}}\n",
                        plan_arm_type(struct_name, arm)
                    ));
                }
                out.push_str(&indented(&format!(
                    "switch _b := {VARIANT_LOCAL}.(type) {{\n{cases}\
                     default:\n\
                     \tpanic(\"{struct_name}: the source presents no arm of its variant\")\n\
                     }}\n"
                )));
            }
        }
    }
    out.push_str("\treturn nil\n}");
    // Every refusal the totality analysis names is one the encode returns
    // (docs/adr/0065 §2).
    plan.totality.assert_all_written(struct_name);
    out
}

/// One measure of a derived number as the encode reads it.
struct MeasureText {
    /// The number, an `int`, which is read only where the measure is there.
    size: String,
    /// Where the measure may be absent, the test that it is there.
    present: Option<String>,
}

/// A measure of a derived number, read off the source: the bytes of a byte
/// string or a text, less or more its `sce:length-arith`, tested present
/// where a gate may leave it out.
fn plain_measure(reads: &SourceReads<'_, '_>, measure: &Measure<'_>) -> MeasureText {
    let (field, arith) = match &measure.of {
        Measured::Bytes { field, offset } => (field, length_arith(offset)),
        // How many entries the list holds.
        Measured::Entries { list } => (list, String::new()),
        _ => unreachable!("`unlowered` prints a length of bytes, texts or bodies, or a count"),
    };
    MeasureText {
        size: format!("{}.Len(){arith}", reads.value(&field.id)),
        present: measure.gated.then(|| reads.present(&field.id)),
    }
}

/// The largest value the integer type `ty` holds, or `None` for another type.
fn largest_of(ty: &SceType) -> Option<u64> {
    let bits = ty.int_bit_width()?;
    let bits = if ty.is_signed() { bits - 1 } else { bits };
    Some(if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    })
}

/// The name the encode's locals of the derived `element` are named by.
fn derived_local(l: &LangCtx, element: &ElementId) -> String {
    match element {
        ElementId::Field(id) => l.codec_field_id(id),
        ElementId::Flag { field, flag } => format!(
            "{}{}",
            l.codec_field_id(field),
            filters::to_pascal_case(flag.clone())
        ),
        ElementId::Variant => unreachable!("the plan derives nothing into the variant"),
    }
}

/// The statements, written into `out`, that make the derived number of
/// `element` from what it measures, as the Rust encode's `combine` makes it
/// (docs/adr/0081 §1): the first measure always there fixes it, and every
/// other one there must say the same, or the value is refused with
/// `codec.ErrInconsistentSource`; where every measure may be absent, it is the
/// first one there. A field holds it as `_d<Field>`, refused with
/// `codec.ErrLengthOutOfRange` where it may not fit; where nothing it
/// measures is there, the field holds zero or what the value states. A number
/// held in a flag is answered as the carrier and the statement that folds it
/// into the carrier's local.
fn derive_number(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    reads: &SourceReads<'_, '_>,
    element: &ElementId,
    into: &NumberInto<'_>,
    measures: &[MeasureText],
    out: &mut String,
) -> Option<(String, String)> {
    let named = element.to_string();
    let disagree = || refusal(plan, RefusalKind::InconsistentSource, &named);
    let there = |m: &MeasureText| {
        m.present
            .clone()
            .expect("a measure that may be absent has a test that it is there")
    };
    // The number, and where it may not be there, the test that it is.
    let (size, present) = match measures {
        // Nothing measured: zero, as the Rust encode's `combine` holds it.
        [] => ("0".to_string(), None),
        [one] => (one.size.clone(), one.present.clone()),
        _ => {
            let always: Vec<&MeasureText> =
                measures.iter().filter(|m| m.present.is_none()).collect();
            let gated: Vec<&MeasureText> =
                measures.iter().filter(|m| m.present.is_some()).collect();
            if let Some((anchor, others)) = always.split_first() {
                for other in others {
                    out.push_str(&format!(
                        "\tif {} != {} {{\n\t\t{}\n\t}}\n",
                        other.size,
                        anchor.size,
                        disagree()
                    ));
                }
                for g in &gated {
                    out.push_str(&format!(
                        "\tif {} && {} != {} {{\n\t\t{}\n\t}}\n",
                        there(g),
                        g.size,
                        anchor.size,
                        disagree()
                    ));
                }
                (anchor.size.clone(), None)
            } else {
                let local = derived_local(l, element);
                let (first, found) = (format!("_f{local}"), format!("_h{local}"));
                out.push_str(&format!("\t{first}, {found} := 0, false\n"));
                for (i, g) in gated.iter().enumerate() {
                    let keyword = if i == 0 { "\tif" } else { " else if" };
                    out.push_str(&format!(
                        "{keyword} {} {{\n\t\t{first}, {found} = {}, true\n\t}}",
                        there(g),
                        g.size
                    ));
                }
                out.push('\n');
                for g in gated.iter().skip(1) {
                    out.push_str(&format!(
                        "\tif {} && {} != {first} {{\n\t\t{}\n\t}}\n",
                        there(g),
                        g.size,
                        disagree()
                    ));
                }
                (first, Some(found))
            }
        }
    };
    let (field, narrow, absent) = match (element, into) {
        (
            ElementId::Field(_),
            NumberInto::Field {
                field,
                narrow,
                absent,
            },
        ) => (field, narrow, absent),
        // Folded into its carrier in place of the bits the value presents,
        // where it is there; where it is free and nothing it measures is
        // there, the carrier keeps the value's bits. A number the flag's bits
        // cannot hold is no frame, where the grammar does not bound it within
        // them.
        (ElementId::Flag { field, flag }, NumberInto::Flag) => {
            let def = flag_def(plan, field, flag);
            let width_mask = flag_width_mask(def);
            let mask = width_mask << def.bit;
            let c = carrier_local(l, field);
            let carrier_type = plan
                .model
                .fields
                .iter()
                .find(|f| f.id == *field)
                .map(|carrier| l.type_name(&carrier.sce_type).into_owned())
                .expect("a number held in a flag is held in a flag of its carrier");
            let mut set = String::new();
            if plan.totality.refuses(RefusalKind::LengthOutOfRange, &named) {
                set.push_str(&format!(
                    "\tif {size} > 0x{width_mask:X} {{\n\t\t{}\n\t}}\n",
                    refusal(plan, RefusalKind::LengthOutOfRange, &named)
                ));
            }
            set.push_str(&format!(
                "\t{c} = ({c} &^ 0x{mask:X}) | ({carrier_type}({size}) << {})\n",
                def.bit
            ));
            let statement = match &present {
                None => set,
                Some(there) => format!("\tif {there} {{\n{}\t}}\n", indented(&set)),
            };
            return Some((field.clone(), statement));
        }
        _ => unreachable!("a number goes into the field or the flag it names"),
    };
    let name = l.codec_field_id(&field.id);
    let ty = l.type_name(&field.sce_type);
    // A number past what the field holds is no frame (docs/adr/0065 §5).
    if *narrow == Narrow::Checked {
        let most = largest_of(&field.sce_type)
            .expect("a length that may not fit its field is held in an integer field");
        let past = format!("{size} > 0x{most:X}");
        let test = match &present {
            Some(there) => format!("{there} && {past}"),
            None => past,
        };
        out.push_str(&format!(
            "\tif {test} {{\n\t\t{}\n\t}}\n",
            refusal(plan, RefusalKind::LengthOutOfRange, &named)
        ));
    }
    let d = format!("_d{name}");
    match &present {
        None => out.push_str(&format!("\t{d} := {ty}({size})\n")),
        // Where the content is not, zero, the field gated beside it and not
        // written, what the value states, or what the arm of the group the
        // value holds without the content holds (docs/adr/0065 §6 R4b), zero
        // where it holds neither.
        Some(there) => {
            out.push_str(&format!(
                "\tvar {d} {ty}\n\tif {there} {{\n\t\t{d} = {ty}({size})\n\t}}"
            ));
            if matches!(absent, Absent::Stated | Absent::Group) {
                out.push_str(&format!(
                    " else if _s, ok := {}.Get(); ok {{\n\t\t{d} = _s\n\t}}",
                    reads.call(&field.id)
                ));
            }
            out.push('\n');
        }
    }
    // A value that states the number beside content says it twice, and must
    // say what the content does.
    if *absent == Absent::Stated {
        let there = present
            .as_deref()
            .expect("a number the value states is free: every measure may be absent");
        out.push_str(&format!(
            "\tif _s, ok := {}.Get(); ok && {there} && _s != {d} {{\n\t\t{}\n\t}}\n",
            reads.call(&field.id),
            disagree()
        ));
    }
    None
}
