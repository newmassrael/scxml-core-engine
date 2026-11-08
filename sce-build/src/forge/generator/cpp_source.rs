// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ printer of a codec's encode plan (docs/adr/0081 §2, step 4).
//!
//! A C++ value presents its elements through accessor functions and is made
//! only by its constructor; the encode is a template over any source of the
//! codec, reading each element through a customization point (the member
//! function where the type has one, otherwise a function its namespace
//! declares) and checked by a C++17 trait (ADR 0059 §3). The printer reads the
//! model only through the plan, and lowers the plan's forms one landing at a
//! time (§6): [`unlowered`] names the first form a codec's plan uses that it
//! does not print yet, and such a codec keeps the template's encode whole.

use super::*;
use crate::forge::codec_roles::{ElementId, GateGroup, GateVar, GroupArm, InputSource, InputValue};
use crate::forge::encode_plan::{
    list_law, Absent, Automaton, Bound, Clause, CodecPlan, Derive, EncodePlan, EntryId, Guard,
    HandedFrom, Measured, Narrow, Number, NumberInto, Offset, Presence, Value, Write,
};

/// The first form of `plan` the C++ printer does not print yet, or `None`
/// where it prints the codec whole. `imports` are the codec's own, which say
/// whether a codec it holds is printed too.
pub(super) fn unlowered(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Option<String> {
    unlowered_forms(plan, imports).into_iter().next()
}

/// Every form of `plan` the C++ printer does not print yet, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole. A codec
/// moves at the landing that lowers the last of them, so a landing is chosen
/// by the codecs whose every form it lowers.
pub(super) fn unlowered_forms(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    let plan = match plan {
        CodecPlan::Positional(plan) => plan,
        CodecPlan::Cbor(_) => return vec!["a CBOR codec".to_string()],
        CodecPlan::ContentLine(_) => return vec!["a content-line codec".to_string()],
    };
    let m = plan.model;
    let mut forms = Vec::new();
    // A variant is presented as the arm the value holds, each arm's body a
    // source of its codec, and written by that codec's encode; its tag is
    // derived from the arm into a field of the codec's own, or, where the arm's
    // own header holds it (a peek byte), written by the arm's codec with the
    // rest of the arm. One whose caller hands the tag, or one with an arm a
    // build feature decides, is not yet.
    if let Some(variant) = &m.variant {
        if variant.tag_field.is_none() {
            forms.push("a variant whose caller hands its tag".to_string());
        }
        for arm in &plan.arms {
            if arm.feature.is_some() {
                forms.push(format!("`{}`, an arm a build feature decides", arm.codec));
            }
            if !held_printed(imports, arm.codec) {
                forms.push(format!(
                    "`{}`, an arm of a codec the printer does not print",
                    arm.codec
                ));
            }
            if plan
                .handed
                .get(arm.codec)
                .is_some_and(|handed| handed.iter().any(|h| h.from.is_none()))
            {
                forms.push(format!(
                    "`{}`, an arm handed an input nothing binds",
                    arm.codec
                ));
            }
        }
    }
    for element in &plan.elements {
        let id = &element.field.id;
        match &element.value {
            Value::Scalar(t) if t.int_bit_width().is_some() => {}
            // A value of an imported enum is presented as the enum's type and
            // written as its carrier; the decode reads the carrier into the
            // type, refusing one a closed set does not declare.
            Value::Scalar(SceType::Enum(_)) => {}
            Value::Scalar(_) => forms.push(format!(
                "`{id}`, which is not an integer, a flag set or an enum"
            )),
            // A byte string is presented within its bound (docs/adr/0077 §3),
            // and a text likewise, in UTF-8 bytes; one the protocol states no
            // bound of is held to `kUnbounded` (§7).
            Value::Bytes(_) | Value::Text(_) => {}
            // A body or a list of entries ([`held_codec_forms`]); one a gate
            // decides is presented as an optional value and written where it
            // is present.
            Value::Body { .. } | Value::List { .. } => {
                forms.extend(held_codec_forms(plan, imports, id, &element.value));
            }
        }
        // A gated element is presented as an optional value; a number the
        // value states only while its content is absent is not yet.
        if element.presence == Presence::Stated {
            forms.push(format!(
                "`{id}`, a number the value states where its content is absent"
            ));
        }
    }
    // A group (docs/adr/0065 §6 R4b, R4c) is presented as the class of the
    // members of its one side, held optionally, or as a variant of a class for
    // each of its arms; its members are integers, byte strings, texts, bodies
    // or lists, its gates flag inputs or one-bit flags of the codec's own. The
    // encode keeps a body or a list as the arms that hold it present it, all
    // as one type, and two lists of one count as one `Aligned` of the two (R5);
    // one held to a law is not yet, nor is a group of a chain's memberships, or
    // one that holds an enum.
    for group in plan.groups() {
        // A chain's memberships make arms that each hold the chain as a `Chain`
        // of the kinds the arm says (R6b), optionally where a gate of its own
        // may leave it out, and a bare entry's the arm where its gate holds
        // as the entry's place among the others (R6a).
        for id in group.fields() {
            let member = m
                .fields
                .iter()
                .find(|f| f.id == id)
                .expect("a group's member is a field of the codec");
            let value = plan.value(member);
            match &value {
                Value::Scalar(t) if t.int_bit_width().is_some() => {}
                Value::Bytes(_) | Value::Text(_) => {}
                Value::Body { .. } | Value::List { .. } => {
                    forms.extend(held_codec_forms(plan, imports, id, &value));
                    if matches!(&value, Value::List { law, .. } if !law.is_none()) {
                        forms.push(format!("`{id}`, a list a group holds held to a law"));
                    }
                }
                Value::Scalar(_) => forms.push(format!(
                    "`{id}`, a member of a group that is not an integer, a byte string, a text, \
                     a body or a list"
                )),
            }
        }
    }
    forms.extend(plan.derivations.iter().filter_map(derivation_unlowered));
    // Each field is an element the value presents, a member of a group it
    // presents, or a number the encode derives whole (docs/adr/0081 §3).
    let grouped = |id: &str| plan.groups().iter().any(|g| g.fields().contains(&id));
    forms.extend(
        m.fields
            .iter()
            .filter(|f| {
                !plan.elements.iter().any(|e| e.field.id == f.id)
                    && !grouped(&f.id)
                    && !derived_number(plan, &f.id)
            })
            .map(|field| format!("`{}`, which no element presents", field.id)),
    );
    // An input the caller hands the encode is a parameter of it. One the value
    // says (docs/adr/0065 §6 R4a) is read off it: off the field gated on it
    // alone, off the group gated on it, off the body always there that says it
    // in every value, off the arm of the variant the value holds (§6 R4d, R4e),
    // or off the start of the list held to it (§6 R8). A body a gate leaves out,
    // or one that says it in some of its values only, is not yet.
    let gated = |id: &str| {
        grouped(id)
            || plan
                .elements
                .iter()
                .any(|e| e.field.id == id && e.presence == Presence::Gated)
    };
    for said in &plan.said {
        let name = &said.input.name;
        match &plan.roles.inputs[name] {
            InputSource::Gate { .. }
            | InputSource::Group
            | InputSource::List { .. }
            | InputSource::EveryArm { .. }
            | InputSource::SomeArms { .. } => {}
            // A body a gate leaves out says nothing of the input it forwards.
            InputSource::Forward { embed, .. } if gated(embed) => {
                forms.push(format!("`{name}`, a flag input a body a gate decides says"))
            }
            InputSource::Forward { partly: false, .. } => {}
            InputSource::Forward { partly: true, .. } => forms.push(format!(
                "`{name}`, a flag input a body says in some of its values only"
            )),
        }
    }
    // A body's said input is read through the body's codec, which says it in
    // every value; one it says in some values only answers nothing at times.
    for derived in &plan.derivations {
        if let Derive::Number(number) = &derived.by {
            for measure in &number.measures {
                if let Measured::Says { codec, input, .. } = &measure.of {
                    let partly = imports.iter().any(|import| {
                        import.alias == *codec && import.codec_partly_says_inputs.contains(input)
                    });
                    if partly {
                        forms.push(format!(
                            "`{}`, an input a body says in some of its values only",
                            derived.element
                        ));
                    }
                }
            }
        }
    }
    if !plan.gate_checks.is_empty() || !plan.input_checks.is_empty() || !plan.totality.is_total() {
        forms.push("an encode that can refuse".to_string());
    }
    let gated_element = |f: &CodecField| gated(&f.id);
    for write in &plan.program {
        match write {
            // A fixed layout has no field a gate decides; were one there, its
            // offset would not be fixed.
            Write::Packed(fields) => forms.extend(
                fields
                    .iter()
                    .filter(|f| gated_element(f))
                    .map(|f| format!("`{}`, a gated field of a fixed layout", f.id)),
            ),
            Write::Fixed(field) | Write::Tail { field, .. } if gated_element(field) => {
                forms.push(format!("`{}`, a gated field of a fixed layout", field.id))
            }
            Write::Fixed(_) | Write::Tail { .. } => {}
            Write::Streamed {
                field,
                guard,
                list: None,
            } => {
                // A streamed field is written whole, in bytes the sink writes.
                if matches!(field.bit_size, BitSize::Fixed { bits } if !matches!(bits, 8 | 16 | 32 | 64))
                {
                    forms.push(format!(
                        "`{}`, a streamed field of less than a byte",
                        field.id
                    ));
                }
                match guard {
                    Guard::Always => {}
                    // Written where the value holds it: an element it presents
                    // as an optional value, or a member of a group it holds.
                    Guard::Present if gated_element(field) => {}
                    Guard::Present => forms.push(format!(
                        "`{}`, a field written where a group holds it",
                        field.id
                    )),
                    // Written where its gate holds, which the printer tests on
                    // the carriers as the encode writes them, on the inputs it
                    // is handed, and on the arm the value holds of a group whose
                    // gate is a chain's membership.
                    Guard::Gate => {
                        let testable = field.present_if.as_ref().is_some_and(|gate| {
                            plan.clauses(gate).iter().all(|c| {
                                matches!(
                                    c,
                                    Clause::Flag { .. }
                                        | Clause::Input { .. }
                                        | Clause::Member {
                                            held: Some((_, Some(_))),
                                            ..
                                        }
                                )
                            })
                        });
                        if !testable {
                            forms.push(format!("`{}`, gated on a chain's membership", field.id));
                        }
                    }
                }
            }
            // A list is walked entry by entry, each entry written whole, a chain
            // setting on each entry but the last the flag its entries end on,
            // and one a gate decides is walked where the value holds it. An
            // entry says the input a chain supplies from the entry before, which
            // the list's law holds it to (docs/adr/0065 §6 R8b); one handed such
            // an input it does not say is not yet. A bare entry's chain is
            // walked as slots, the named entry where the value holds its place
            // (R6a).
            Write::Streamed {
                field,
                guard,
                list: Some(list),
            } if matches!(guard, Guard::Always)
                || (matches!(guard, Guard::Present) && gated_element(field)) =>
            {
                let law = list_law(&plan.roles, &field.id);
                let unsaid = list
                    .previous
                    .iter()
                    .any(|(input, _)| !law.inputs.iter().any(|held| held.input == *input));
                if unsaid {
                    forms.push(format!(
                        "`{}`, a chain whose entries say an input the entry before supplies",
                        field.id
                    ));
                }
            }
            Write::Streamed { field, .. } => forms.push(format!(
                "`{}`, a list written where it is present",
                field.id
            )),
            // The variant is named once, by the model.
            Write::Variant if m.variant.is_some() => {}
            Write::Variant => forms.push("a variant".to_string()),
        }
    }
    // A field is written in its declared bits (a byte or less, or one of the
    // sink's whole widths), as the bytes it holds, as a VLE of a width the
    // sink writes, or as the body of the codec it embeds.
    forms.extend(
        m.fields
            .iter()
            .filter(|f| {
                !matches!(
                    f.bit_size,
                    BitSize::Fixed {
                        bits: 1..=8 | 16 | 32 | 64
                    } | BitSize::Tail
                        | BitSize::LengthRef
                        | BitSize::Embed
                        | BitSize::Repeat { .. }
                        | BitSize::TlvChain { .. }
                        | BitSize::Vle {
                            width_bits: 16 | 32 | 64
                        }
                )
            })
            .map(|field| format!("`{}`, of no width the sink writes whole", field.id)),
    );
    forms
}

/// The member of a codec's `Overrides` that sets `flag` of its carrier
/// `carrier`: the codec's own name for it, which a chain holding the codec
/// spells alike.
fn override_member(l: &LangCtx, carrier: &str, flag: &str) -> String {
    format!(
        "{}_{}",
        l.codec_field_id(carrier),
        filters::to_snake_case(flag.to_string())
    )
}

/// Whether the codec a value holds as `alias` is one the C++ printer prints,
/// whose namespace has the encode over any source of it (`Wire`).
fn held_printed(imports: &[ImportContext], alias: &str) -> bool {
    imports
        .iter()
        .any(|import| import.alias == alias && import.cpp_printed)
}

/// What of `id`, a body or a list of entries the value presents alone or a
/// group of it holds, the printer does not print yet.
///
/// A body is encoded over a source of its codec, which a codec the printer
/// prints has; it is written whole, every input it takes handed by what the
/// parent binds to it. A list of entries is held to its bound, `kUnbounded`
/// where the protocol states none (docs/adr/0077 §7), and walked entry by
/// entry, each written by its codec's encode over the source the list holds it
/// as. One whose entries say the inputs it hands them is held to its law
/// (docs/adr/0065 §6 R8), an entry that says an input in some of its values
/// only agreeing with any state of it from the others (R8b).
fn held_codec_forms(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
    id: &str,
    value: &Value<'_>,
) -> Vec<String> {
    let mut forms = Vec::new();
    let unbound = |codec: &str| {
        plan.handed
            .get(codec)
            .is_some_and(|handed| handed.iter().any(|h| h.from.is_none()))
    };
    match value {
        Value::Body { codec } => {
            if !held_printed(imports, codec) {
                forms.push(format!(
                    "`{id}`, a body of a codec the printer does not print"
                ));
            }
            if unbound(codec) {
                forms.push(format!("`{id}`, a body handed an input nothing binds"));
            }
        }
        Value::List { codec, .. } => {
            if !held_printed(imports, codec) {
                forms.push(format!(
                    "`{id}`, a list of entries of a codec the printer does not print"
                ));
            }
            if unbound(codec) {
                forms.push(format!(
                    "`{id}`, a list whose entries are handed an input nothing binds"
                ));
            }
        }
        Value::Scalar(_) | Value::Bytes(_) | Value::Text(_) => {}
    }
    forms
}

/// The C++ class of the codec a value holds as `alias`, and the namespace that
/// holds its `Wire` and its traits: a generated codec's class is named as its
/// namespace is (`::SCE::Generated::X::X`).
fn held_codec<'i>(imports: &'i [ImportContext], alias: &str) -> (&'i str, &'i str) {
    let import = imports
        .iter()
        .find(|import| import.alias == alias)
        .expect("`unlowered` refuses a body of a codec not imported");
    let class = import.member_type.as_str();
    let namespace = class
        .rsplit_once("::")
        .map(|(namespace, _)| namespace)
        .expect("a generated C++ codec's class is named in its namespace");
    (class, namespace)
}

/// What of the derivation `derived` the printer does not print yet, or `None`
/// for one it does:
///
/// - a flag set exactly when the one field it gates is present, or when the
///   fields one group holds are, which cannot disagree;
/// - how many bytes one byte string holds, as it is or plus or less a constant,
///   into a field it always fits (zero where a gate leaves the bytes out and the
///   field is gated beside them, or what the group that holds the field holds on
///   the side without the bytes) or into a flag of a carrier (which keeps the
///   bits the value presents where the bytes are absent);
/// - how many entries one list holds, into a field or a flag likewise (zero
///   where a gate leaves the list out);
/// - the input a body says where the value holds the body, or the arm the
///   value holds says where it holds one that says it, into a field or a flag
///   likewise.
fn derivation_unlowered(derived: &crate::forge::encode_plan::Derived<'_>) -> Option<String> {
    let element = &derived.element;
    match (element, &derived.by) {
        (ElementId::Flag { .. }, Derive::Presence(presence)) => (presence.of.len() > 1
            && !presence.together)
            .then(|| format!("`{element}`, a flag of fields that may disagree on it")),
        (_, Derive::Number(number)) => number_unlowered(element, number),
        // The tag of the arm the value holds, into the field or the flag that
        // holds it.
        (ElementId::Field(_) | ElementId::Flag { .. }, Derive::VariantTag) => None,
        // The flag a chain's entry is closed by, which the walk of the chain
        // sets on each entry as it writes it.
        (ElementId::Flag { .. }, Derive::ChainContinuation) => None,
        // A flag of a disjunction that decides a group: what the arm the
        // value holds gives it (docs/adr/0065 §6 R4c).
        (ElementId::Flag { .. }, Derive::ArmFlag { .. }) => None,
        // The tag of the arm each embedded body's variant holds, masked to the
        // flag's bits; an arm a build feature decides is not yet.
        (ElementId::Flag { .. }, Derive::ParentTag(tag)) => tag
            .terms
            .iter()
            .any(|term| term.arms.iter().any(|arm| arm.feature.is_some()))
            .then(|| format!("`{element}`, a tag an arm a build feature decides fills")),
        (ElementId::Field(id), _) => Some(format!("`{id}`, a tag or a flag the encode derives")),
        _ => Some(format!("`{element}`, a flag the encode derives")),
    }
}

/// What of the derived number `number` of `element` the printer does not
/// print yet, or `None` for one it does ([`derivation_unlowered`]).
fn number_unlowered(element: &ElementId, number: &Number<'_>) -> Option<String> {
    let [measure] = number.measures.as_slice() else {
        return Some(format!(
            "`{element}`, a number the encode derives from more than one measure"
        ));
    };
    match &measure.of {
        Measured::Bytes {
            offset: Offset::Less { held: false, .. },
            ..
        } => {
            return Some(format!(
                "`{element}`, a length less a constant that may not be held"
            ))
        }
        Measured::Bytes { .. } => {}
        // How many bytes a body encodes to, counted by its encode over a sink
        // that keeps none: a body whose encode can refuse passes the refusal
        // on, which the printer does not yet.
        Measured::Encoded { refuses: false, .. }
            if matches!(number.into, NumberInto::Field { .. }) => {}
        Measured::Encoded { refuses: false, .. } => {
            return Some(format!("`{element}`, an encoded length held in a flag"))
        }
        Measured::Encoded { refuses: true, .. } => {
            return Some(format!(
                "`{element}`, the encoded length of a body that can refuse"
            ))
        }
        // The input a body says, read through the body's codec, where the
        // value holds the body.
        Measured::Says { .. } => {}
        // How many entries a list holds, where the value holds the list.
        Measured::Entries { .. } => {}
        // The input the arm the value holds says, read through the arm's
        // codec, where it holds one that says it (docs/adr/0065 §6 R4a).
        Measured::ArmSays { .. } => {}
        // The start of a list held to a law, which every entry says
        // (docs/adr/0065 §6 R8), where the value holds the list.
        Measured::Start { .. } => {}
    }
    match (element, &number.into) {
        (
            ElementId::Field(_),
            NumberInto::Field {
                narrow: Narrow::Cast | Narrow::Widened,
                absent,
                ..
            },
        ) => match (measure.gated, absent) {
            // Where a gate leaves the content out, the field holds zero, or
            // what the group that holds it holds on that side.
            (false, Absent::Never) | (true, Absent::Zero | Absent::Group) => None,
            _ => Some(format!(
                "`{element}`, a number of content a gate decides that the value states where \
                 it is absent"
            )),
        },
        (ElementId::Field(_), NumberInto::Field { .. }) => {
            Some(format!("`{element}`, a length that may not fit its field"))
        }
        (ElementId::Flag { .. }, NumberInto::Flag) => None,
        _ => Some(format!(
            "`{element}`, a number derived into what holds no number"
        )),
    }
}

/// Whether the plan derives the field `id` whole, as a number or as the tag of
/// the arm the value holds, which the value does not present: the encode
/// computes it as the local `_d_<id>`, and the decode reads it and drops it.
fn derived_number(plan: &EncodePlan<'_>, id: &str) -> bool {
    plan.derivations.iter().any(|derived| {
        matches!(&derived.element, ElementId::Field(element) if element == id)
            && matches!(derived.by, Derive::Number(_) | Derive::VariantTag)
    })
}

/// The text the C++ printer gives the codec's header.
pub(super) struct CppSource {
    /// Before the value's class: the readers of each element, the trait a
    /// source satisfies, and the encode over any source (`Wire`).
    pub(super) before: String,
    /// Inside the class, where the public members were: the constructor and
    /// an accessor for each element.
    pub(super) members: String,
    /// At the end of the class: the elements it holds.
    pub(super) private: String,
    /// In the decode, once each field is read into a local of its name: the
    /// statements that make the value `_decoded` from them, refusing a byte
    /// string outside its bound as the cursor's `LengthOutOfRange`.
    pub(super) decoded: String,
}

/// The name the value's class holds the element `id` under.
pub(super) fn member(id: &str) -> String {
    format!("{id}_")
}

/// Where an input the value says is read: in the encode, off the elements it
/// read once (`_e_<field>`, `_r_<body>`), or in the input's own function a
/// parent calls, off the source it is handed.
#[derive(Clone, Copy)]
enum Read {
    Encode,
    Source,
}

/// The `std::uint8_t` the input `input` that the value says holds
/// (docs/adr/0065 §6 R4a): set exactly when the field gated on it alone is
/// present (absent, for a field gated on its clear state), or what the body
/// that forwards it says, read through that body's codec. [`unlowered`] refuses
/// every other way a value says one. An input the arms of the variant say is
/// read off `_r_body`, the body as the value holds it, which the encode and the
/// input's own function bind before they read it.
fn said_input(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
    elements: &[Presented<'_>],
    groups: &[GroupPart<'_>],
    variant: Option<&VariantPart>,
    input: &str,
    at: Read,
) -> String {
    let element = |id: &str| {
        elements
            .iter()
            .find(|e| e.field.id == id)
            .expect("an input the value says is said by an element it presents")
    };
    match &plan.roles.inputs[input] {
        InputSource::Gate { field, when_clear } => {
            let e = element(field);
            let present = match at {
                Read::Encode => format!("{}.has_value()", e.local()),
                // A body is presented as an optional source of its codec,
                // which is tested as it is.
                Read::Source if e.body.is_some() => {
                    format!("source::{}(src).has_value()", e.name)
                }
                Read::Source => format!(
                    "static_cast<{}>(source::{}(src)).has_value()",
                    e.presented(),
                    e.name
                ),
            };
            let (set, clear) = if *when_clear { (0, 1) } else { (1, 0) };
            format!("static_cast<std::uint8_t>({present} ? {set} : {clear})")
        }
        InputSource::Forward { embed, input, .. } => {
            let e = element(embed);
            let held = e
                .body
                .as_ref()
                .expect("an input forwarded from a body is said by a body");
            let body = match at {
                Read::Encode => format!("_r_{}", e.name),
                Read::Source => format!("source::{}(src)", e.name),
            };
            format!(
                "{}::input_{}({body})",
                held.namespace,
                filters::to_snake_case(input.clone())
            )
        }
        // The value the arm of the group gated on it that the value holds
        // gives it, or the one it has while the value holds no arm.
        InputSource::Group => {
            let gate = GateVar::Input(input.to_string());
            let group = groups
                .iter()
                .find(|g| g.group.gates.contains(&gate))
                .expect("an input a group says has its group");
            let held = match at {
                Read::Encode => format!("_r_{}", group.name),
                Read::Source => format!("source::{}(src)", group.name),
            };
            group.gate_value(&gate, &held)
        }
        // The start of the list held to a law that every entry of says
        // (docs/adr/0065 §6 R8). A list a gate leaves out says nothing: the
        // encode reads it as clear, and a source answers no value.
        InputSource::List {
            list,
            input: entry_input,
            partly,
            ..
        } => {
            let e = element(list);
            let laws = super::rust_source::list_laws(&plan.roles, list);
            let start = |lawful: &str| {
                let start = format!("{lawful}.start()");
                if laws.len() == 1 {
                    format!("static_cast<std::uint8_t>({start})")
                } else {
                    let at = laws
                        .iter()
                        .position(|law| law.binder.input == *entry_input)
                        .expect("the list holds the input by its law");
                    format!("static_cast<std::uint8_t>(std::get<{at}>({start}))")
                }
            };
            let (local, name) = (e.local(), &e.name);
            match (at, partly) {
                (Read::Encode, false) => start(&local),
                (Read::Encode, true) => format!(
                    "({local}.has_value() ? {} : std::uint8_t{{0}})",
                    start(&format!("(*{local})"))
                ),
                (Read::Source, false) => start(&format!("source::{name}(src)")),
                (Read::Source, true) => format!(
                    "(source::{name}(src).has_value() ? std::optional<std::uint8_t>({}) : std::nullopt)",
                    start(&format!("(*source::{name}(src))"))
                ),
            }
        }
        // What the arm the value holds says, read through that arm's codec
        // (docs/adr/0065 §6 R4d). A body that holds no arm says nothing, as its
        // write writes nothing.
        InputSource::EveryArm { arms } => {
            let said = arm_said_input(imports, variant, arms, false);
            format!("({said}std::uint8_t{{0}})")
        }
        // As above, and nothing from an arm that does not read the input: it
        // has no bit the input could change (docs/adr/0065 §6 R4e). No gate of
        // the codec reads such an input, so only a parent asks for it.
        InputSource::SomeArms { arms } => match at {
            Read::Source => {
                let said = arm_said_input(imports, variant, arms, true);
                format!("({said}std::optional<std::uint8_t>{{}})")
            }
            Read::Encode => {
                unreachable!("no gate reads an input some arms of the variant say")
            }
        },
    }
}

/// `_r_body.index() == i ? <said> : ` for each arm `i` of the variant whose
/// codec `arms` binds an input of, `<said>` being that input as the codec's
/// `input_<name>` reads it off the arm; held `optional`, an arm whose codec says
/// it in every value is wrapped as one.
fn arm_said_input(
    imports: &[ImportContext],
    variant: Option<&VariantPart>,
    arms: &std::collections::BTreeMap<String, String>,
    optional: bool,
) -> String {
    let variant = variant.expect("an input the arms say is said by a codec with a variant");
    variant
        .arms
        .iter()
        .enumerate()
        .filter_map(|(index, (held, tag))| {
            let input = arms.get(&held.alias)?;
            let arm = match tag {
                Some(_) => format!("*std::get_if<{index}>(&_r_body)"),
                None => format!("std::get_if<{index}>(&_r_body)->body"),
            };
            let read = format!(
                "{}::input_{}({arm})",
                held.namespace,
                filters::to_snake_case(input.clone())
            );
            let partly = imports.iter().any(|import| {
                import.alias == held.alias && import.codec_partly_says_inputs.contains(input)
            });
            let read = if optional && !partly {
                format!("std::optional<std::uint8_t>({read})")
            } else {
                read
            };
            Some(format!("_r_body.index() == {index} ? {read} : "))
        })
        .collect()
}

/// The identifier of `entry`, a source of the entry codec of the chain `chain`
/// whose namespace is `ns`, as a `std::uint64_t`: the flag or field the chain's
/// `entry-id` names, read through the entry codec's readers, less the flags it
/// leaves out.
fn entry_identifier(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    ns: &str,
    chain: &str,
    entry: &str,
) -> String {
    match plan
        .entry_ids
        .get(chain)
        .expect("a chain whose entries a gate or a law tells apart declares entry-id")
    {
        EntryId::Flag { carrier, bit, mask } => format!(
            "((static_cast<std::uint64_t>({ns}::source::{}({entry})) >> {bit}) & 0x{mask:X}u)",
            l.codec_field_id(carrier)
        ),
        EntryId::Whole {
            field,
            mask: Some(mask),
        } => format!(
            "(static_cast<std::uint64_t>({ns}::source::{}({entry})) & 0x{mask:X}u)",
            l.codec_field_id(field)
        ),
        EntryId::Whole { field, mask: None } => format!(
            "static_cast<std::uint64_t>({ns}::source::{}({entry}))",
            l.codec_field_id(field)
        ),
    }
}

/// The member of the key of the kind `key` of a bare entry of the chain
/// `chain` (docs/adr/0065 §6 R6a) that makes the entry of the kind: its
/// carrier, the identifier, and, where the entry codec has a variant, the arm
/// the identifier's tag bits select, value-made, as the Rust key's `named()`.
fn named_entry(plan: &EncodePlan<'_>, imports: &[ImportContext], chain: &str, key: u64) -> String {
    let field = plan
        .model
        .fields
        .iter()
        .find(|f| f.id == chain)
        .expect("a membership names a chain of the codec");
    let Value::List { codec, .. } = plan.value(field) else {
        unreachable!("the chain whose memberships are gates is a list")
    };
    let (class, _) = held_codec(imports, codec);
    let import = imports
        .iter()
        .find(|i| i.alias == codec)
        .expect("a chain imports its entry codec");
    let body = match &import.codec_variant {
        None => String::new(),
        Some(variant) => {
            let carrier = import
                .codec_fields
                .first()
                .expect("a bare entry is its carrier");
            let flag = carrier
                .flags
                .iter()
                .find(|f| variant.tag_flag.as_deref() == Some(f.name.as_str()))
                .expect("a bare entry's tag is a flag of its carrier");
            let mask = if flag.width >= 64 {
                u64::MAX
            } else {
                (1u64 << flag.width) - 1
            };
            let tag = key.checked_shr(flag.bit).unwrap_or(0) & mask;
            let index = variant
                .arms
                .iter()
                .position(|a| a.value == tag)
                .expect("a bare entry's tag selects an arm");
            format!(", {class}Variant{{std::in_place_index<{index}>}}")
        }
    };
    format!(
        "\x20   /// The entry of the kind: its identifier, and nothing else to say.\n\
         \x20   [[nodiscard]] static {class} named() {{\n\
         \x20       return {class}{{0x{key:X}{body}}};\n\
         \x20   }}\n"
    )
}

/// The types each list held to a law is held by (docs/adr/0065 §6 R8), in the
/// codec's namespace: for each input its entries say, the type that reads it
/// off any source of an entry through the entry codec's `input_<name>`; for
/// each kind of entry a chain supplies an input after, the type that tells an
/// entry of that kind by its identifier; and the law, the runtime's automaton
/// for each input, several run together as `Laws`.
fn law_definitions(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
    elements: &[Presented<'_>],
    l: &LangCtx,
    struct_name: &str,
) -> String {
    let mut out = String::new();
    for e in elements.iter().filter(|e| e.law.is_some()) {
        let field = &e.field.id;
        let (held, _) = e.list.as_ref().expect("a list held to a law is a list");
        let ns = &held.namespace;
        let mut keys: Vec<u64> = Vec::new();
        for law in super::rust_source::list_laws(&plan.roles, field) {
            let input = &law.binder.input;
            let says = super::rust_source::says_type(struct_name, field, input);
            let read = format!(
                "{ns}::input_{}(entry)",
                filters::to_snake_case(input.clone())
            );
            // An entry that says the input in some of its values only answers
            // nothing from the others, which disagrees with no state (R8b).
            let partly = imports.iter().any(|import| {
                import.alias == law.binder.alias && import.codec_partly_says_inputs.contains(input)
            });
            let said = if partly {
                format!(
                    "const std::optional<std::uint8_t> said = {read};\n\
                     \x20       return said.has_value() ? std::optional<bool>(*said != 0) : std::nullopt;"
                )
            } else {
                format!("return {read} != 0;")
            };
            out.push_str(&format!(
                "/// The input `{input}` each entry of `{field}` says, which the list hands every\n\
                 /// entry as {} and holds to its law (docs/adr/0065 §6 R8).\n\
                 struct {says} {{\n\
                 \x20   template <class E>\n\
                 \x20   [[nodiscard]] static std::optional<bool> says(const E& entry) {{\n\
                 \x20       {said}\n\
                 \x20   }}\n\
                 }};\n\n",
                law.against,
            ));
            if let InputValue::PreviousEntry { id } = law.against {
                if !keys.contains(&id) {
                    keys.push(id);
                }
            }
        }
        for id in keys {
            let key = super::rust_source::key_type(struct_name, field, id);
            let identifier = entry_identifier(plan, l, ns, field, "entry");
            out.push_str(&format!(
                "/// The kind of entry after which `{field}` supplies its input set: an entry\n\
                 /// whose identifier is 0x{id:X} (docs/adr/0065 §6 R8b).\n\
                 struct {key} {{\n\
                 \x20   template <class E>\n\
                 \x20   [[nodiscard]] static bool matches(const E& entry) {{\n\
                 \x20       return {identifier} == 0x{id:X}u;\n\
                 \x20   }}\n\
                 }};\n\n"
            ));
        }
        let automata: Vec<String> = list_law(&plan.roles, field)
            .inputs
            .iter()
            .map(|input| {
                let says = super::rust_source::says_type(struct_name, field, &input.input);
                match input.automaton {
                    Automaton::Uniform => format!("::SCE::Forge::Uniform<{says}>"),
                    Automaton::AfterKey { id } => format!(
                        "::SCE::Forge::AfterKey<{says}, {}>",
                        super::rust_source::key_type(struct_name, field, id)
                    ),
                }
            })
            .collect();
        let definition = match automata.as_slice() {
            [one] => one.clone(),
            all => format!("::SCE::Forge::Laws<{}>", all.join(", ")),
        };
        out.push_str(&format!(
            "/// The law the entries of `{field}` follow in the inputs they say, which the\n\
             /// list holds them to (docs/adr/0065 §6 R8).\n\
             using {} = {definition};\n\n",
            e.law.as_ref().expect("filtered to a list held to a law")
        ));
    }
    out
}

/// The start the decode holds the list `field`, viewed as `view`, to its law
/// from (docs/adr/0065 §6 R8): for each input the list hands its entries, the
/// value it hands them, read off the carrier or the input the decode read;
/// none for an input a chain supplies from the entry before, whose start is
/// fixed. A law of one input starts from its own; several, from each one's.
fn decoded_law_start(plan: &EncodePlan<'_>, l: &LangCtx, field: &str, view: &str) -> String {
    let starts: Vec<String> = super::rust_source::list_laws(&plan.roles, field)
        .iter()
        .map(|law| match &law.against {
            InputValue::Carrier { carrier, flag } => {
                let bit = plan
                    .model
                    .fields
                    .iter()
                    .find(|f| f.id == *carrier)
                    .and_then(|f| f.flags.iter().find(|fl| fl.name == *flag))
                    .map(|fl| fl.bit)
                    .expect("a list's bound flag is a flag of the codec's carrier");
                format!("((({} >> {bit}) & 0x1) != 0)", l.codec_field_id(carrier))
            }
            InputValue::Input { name } => {
                format!("(({} & 0x01) != 0)", filters::to_snake_case(name.clone()))
            }
            InputValue::PreviousEntry { .. } => "::SCE::Forge::FixedStart{}".to_string(),
        })
        .collect();
    match starts.as_slice() {
        [one] => one.clone(),
        all => format!("{view}::start_type{{{}}}", all.join(", ")),
    }
}

/// A bit-range of a carrier the encode derives, folded into the carrier's
/// value before the carrier is written.
struct Fold {
    /// The flag.
    flag: String,
    /// What the bits hold, as a `std::uint64_t`.
    bits: String,
    /// Where it is a number of bytes a gate may leave out, the test that they
    /// are there; the carrier keeps the bits the value presents where they are
    /// not.
    present: Option<String>,
}

/// One element a printed value presents.
struct Presented<'p> {
    field: &'p CodecField,
    /// The name of its accessor, of its reader in `source`, and of its
    /// constructor's parameter.
    name: String,
    /// What it is presented as where it is present.
    view: String,
    /// What the value holds it as where it is present.
    held: String,
    /// Whether it is a byte string or a text, held to its bound.
    bytes: bool,
    /// Whether it is a text, whose UTF-8 bytes are `char`s.
    text: bool,
    /// Whether a gate decides it, so that it is presented as an optional
    /// value (docs/adr/0055).
    optional: bool,
    /// Where it is the body of a codec it holds, that codec.
    body: Option<HeldCodec>,
    /// Where it is a list of entries of a codec, that codec and the list's
    /// bounds, as `Max` or `Max, Min`.
    list: Option<(HeldCodec, String)>,
    /// Where it is a list whose entries say inputs it hands them, the type of
    /// the law it holds them to (docs/adr/0065 §6 R8): it is then viewed and
    /// held as a `::SCE::Forge::Lawful` of that law.
    law: Option<String>,
    /// Where it is two lists of one count an arm of a group holds as one
    /// member (docs/adr/0065 §6 R5), the two lists: it is then viewed and held
    /// as a `::SCE::Forge::Aligned` of them.
    aligned: Option<AlignedLists>,
    /// Where it is the chain an arm of the gates on its entries holds
    /// (docs/adr/0065 §6 R6b), the kinds the arm says of it: it is then viewed
    /// and held as a `::SCE::Forge::Chain` of those kinds.
    chain: Option<ChainKinds>,
}

/// The kinds of entry the chain one arm of a group of its memberships holds
/// has an entry of, and has none of (docs/adr/0065 §6 R6b).
struct ChainKinds {
    /// The key type of each kind it holds an entry of.
    holds: Vec<String>,
    /// The key type of each kind it holds no entry of.
    lacks: Vec<String>,
    /// The trait that says whether a type is such a chain a source may present.
    trait_name: String,
    /// The chain's bounds, the most and the fewest, as `WithinBounds` reads them.
    most: String,
    least: u64,
    /// The namespace of the chain's entry codec.
    namespace: String,
    /// The value class of the chain's entry codec.
    entry: String,
    /// The view of the chain itself, which the decode makes of what it read.
    view: String,
    /// Whether the arm may hold no chain: the chain is gated itself, and the
    /// arm holds no kind of entry, which an absent chain holds none of either.
    /// It is then viewed and held as a `std::optional` of the chain.
    optional: bool,
    /// Whether the arm is the one where the gate on a bare entry holds
    /// (docs/adr/0065 §6 R6a): it then holds the other entries and the named
    /// entry's place, a `::SCE::Forge::Placed` of the one kind it holds.
    placed: bool,
}

impl ChainKinds {
    /// Whether the arm's chain has a value made by default: one that must
    /// hold no kind holds the fewest entries value-made, and a bare entry's
    /// place the named entry alone; one that must hold an entry of some kind
    /// has none.
    fn has_default(&self) -> bool {
        self.holds.is_empty() || self.placed
    }

    /// What the encode keeps of the chain the arm holds: the others of a bare
    /// entry's place, or the list a chain holds.
    fn list_read(&self) -> &'static str {
        if self.placed {
            "rest()"
        } else {
            "inner()"
        }
    }

    /// The trait that says whether a type is what a source may present for
    /// the chain in the arm: the chain, or an optional of it.
    fn presented_trait(&self) -> String {
        if self.optional {
            format!("{}_present", self.trait_name)
        } else {
            self.trait_name.clone()
        }
    }
}

/// The two lists of one count an arm of a group holds as one member
/// (docs/adr/0065 §6 R5): each list's member of the group, and the namespace
/// of each list's entry codec.
struct AlignedLists {
    first: String,
    second: String,
    first_namespace: String,
    second_namespace: String,
}

/// The variant of a printed codec: the type the value holds the arm in (the
/// template's `{Codec}Variant`, a `std::variant` of each arm's value in the
/// plan's order, the catch-all `{Codec}Default` last), each arm's codec and the
/// tag it names (`None` for the catch-all, which holds its tag), and the arm a
/// value made by default holds.
struct VariantPart {
    type_name: String,
    arms: Vec<(HeldCodec, Option<u64>)>,
    default_index: Option<usize>,
}

impl VariantPart {
    fn of(plan: &EncodePlan<'_>, imports: &[ImportContext], struct_name: &str) -> Option<Self> {
        let variant = plan.model.variant.as_ref()?;
        let arms = plan
            .arms
            .iter()
            .map(|arm| {
                let (_, namespace) = held_codec(imports, arm.codec);
                (
                    HeldCodec {
                        namespace: namespace.to_string(),
                        alias: arm.codec.to_string(),
                    },
                    arm.tag,
                )
            })
            .collect();
        Some(VariantPart {
            type_name: format!("{struct_name}Variant"),
            arms,
            default_index: variant.arms.iter().position(|arm| arm.is_default),
        })
    }

    /// The trait, of the codec's namespace, that says whether what a source
    /// presents as the body is a `std::variant` of a source of each arm's codec
    /// in turn, the catch-all one holding its `tag` and its `body`.
    fn traits(&self) -> String {
        let n = self.arms.len();
        let mut out = String::new();
        let mut each: Vec<String> = Vec::new();
        for (index, (held, tag)) in self.arms.iter().enumerate() {
            let alt = format!("std::variant_alternative_t<{index}, R>");
            match tag {
                Some(_) => each.push(format!("{}::is_source<{alt}>", held.namespace)),
                None => {
                    out.push_str(&format!(
                        "/// Whether `D` is the catch-all arm of a source's body: the tag it holds,\n\
                         /// and its body, a source of the catch-all's codec.\n\
                         template <class D, class = void>\n\
                         struct is_default_arm : std::false_type {{}};\n\
                         template <class D>\n\
                         struct is_default_arm<D, std::void_t<decltype(std::declval<const D&>().tag), decltype(std::declval<const D&>().body)>>\n\
                         \x20   : std::conjunction<std::is_convertible<decltype(std::declval<const D&>().tag), std::uint64_t>,\n\
                         \x20                      {}::is_source<std::decay_t<decltype(std::declval<const D&>().body)>>> {{}};\n\n",
                        held.namespace
                    ));
                    each.push(format!("is_default_arm<{alt}>"));
                }
            }
        }
        out.push_str(&format!(
            "/// Whether `R` is a body a source may present: the arm it holds, of a\n\
             /// `std::variant` of a source of each arm's codec in turn.\n\
             template <class R, class = void>\n\
             struct is_body : std::false_type {{}};\n\
             template <class R>\n\
             struct is_body<R, std::enable_if_t<::SCE::Forge::Detail::IsVariantOf<R, {n}>::value>>\n\
             \x20   : std::conjunction<{}> {{}};\n\n",
            each.join(", ")
        ));
        out
    }

    /// Whether reading the body of a source `S`, and every read of the arm it
    /// holds as a source of that arm's codec, throws nothing.
    fn nothrow_requirement(&self) -> String {
        let read = "source::body(std::declval<const S&>())";
        let body = format!("std::decay_t<decltype({read})>");
        let mut parts = vec![format!("noexcept({read})")];
        for (index, (held, tag)) in self.arms.iter().enumerate() {
            let alt = format!("std::variant_alternative_t<{index}, {body}>");
            parts.push(match tag {
                Some(_) => format!("{}::is_nothrow_source<{alt}>::value", held.namespace),
                None => format!(
                    "{}::is_nothrow_source<std::decay_t<decltype(std::declval<const {alt}&>().body)>>::value",
                    held.namespace
                ),
            });
        }
        parts.join(" && ")
    }
}

/// A codec a printed value holds as a body (its class is the element's view):
/// the namespace whose `Wire` encodes it over any source of it and whose
/// `is_source` says what one is, and the alias the value imports it as.
struct HeldCodec {
    namespace: String,
    alias: String,
}

/// A gate group (docs/adr/0065 §6 R4b, R4c) a printed value holds as one
/// element. Each arm is a class of the members present under one assignment of
/// the gates. A group of one side is its one arm's class, held while the gates
/// are on that side and not otherwise; a sum is a `std::variant` of its arms'
/// classes, in the plan's order, which a value always holds one of, or holds
/// optionally where the gates allow no arm. A source presents it as the same
/// shape of types that each present an arm's members by name, as the value
/// presents a body, and the encode derives the gates from which arm is there.
struct GroupPart<'p> {
    group: &'p GateGroup,
    /// The name of its accessor, of its reader in `source`, and of its
    /// constructor's parameter.
    name: String,
    /// The type it is held as where it is there: its one arm's class, or the
    /// variant of its arms' classes.
    class: String,
    /// Its arms, in the plan's order.
    arms: Vec<ArmPart<'p>>,
    /// Every member of every arm once, in declaration order, each an integer
    /// or a view of a byte string or a text the encode binds to an optional
    /// local (`_e_<member>`), or a body or a list it keeps as read
    /// (`_r_<member>`), from the arm the value holds where that arm holds it.
    members: Vec<Presented<'p>>,
    /// The place of its first member among the codec's fields, where the
    /// value presents it among its elements.
    at: usize,
    /// The class of the codec's value, which names the kinds of entry its
    /// chains' gates ask for.
    struct_name: String,
}

/// One arm of a [`GroupPart`]: the class of the members present while the
/// gates hold the arm's values.
struct ArmPart<'p> {
    arm: &'p GroupArm,
    class: String,
    /// The trait that says whether a type presents each of its members.
    members_trait: String,
    /// Its members, in declaration order.
    members: Vec<Presented<'p>>,
}

impl<'p> GroupPart<'p> {
    fn of(
        plan: &'p EncodePlan<'_>,
        group: &'p GateGroup,
        l: &LangCtx,
        imports: &[ImportContext],
        struct_name: &str,
    ) -> Self {
        let fields = &plan.model.fields;
        let presented = |id: &str| {
            let field = fields
                .iter()
                .find(|f| f.id == id)
                .expect("a group's member is a field of the codec");
            let element = crate::forge::encode_plan::Element {
                field,
                value: plan.value(field),
                presence: Presence::Gated,
            };
            Presented::of(&element, l, imports, None)
        };
        let mut ids = group.fields();
        ids.sort_by_key(|id| fields.iter().position(|f| f.id == *id));
        let members: Vec<Presented<'p>> = ids.into_iter().map(presented).collect();
        let at = members
            .iter()
            .filter_map(|m| fields.iter().position(|f| f.id == m.field.id))
            .min()
            .expect("a group holds a member");
        let name = super::rust_source::group_accessor(group);
        let class = super::rust_source::group_type(struct_name, group);
        // Two lists of one count are one member of an arm that holds both
        // (docs/adr/0065 §6 R5), where the first is, held as an `Aligned` of
        // the two as each list is held.
        let aligned = |first: &str, second: &str| {
            let (one, two) = (presented(first), presented(second));
            let namespace = |p: &Presented<'_>| {
                p.list
                    .as_ref()
                    .map(|(held, _)| held.namespace.clone())
                    .expect("two lists of one count are lists")
            };
            Presented {
                field: one.field,
                name: super::rust_source::aligned_name(&one.name, &two.name),
                view: format!("::SCE::Forge::Aligned<{}, {}>", one.view, two.view),
                held: format!("::SCE::Forge::Aligned<{}, {}>", one.held, two.held),
                bytes: false,
                text: false,
                optional: one.optional,
                body: None,
                list: None,
                law: None,
                aligned: Some(AlignedLists {
                    first_namespace: namespace(&one),
                    second_namespace: namespace(&two),
                    first: one.name,
                    second: two.name,
                }),
                chain: None,
            }
        };
        // The chain whose memberships are gates of the group is held in each
        // arm as a `Chain` of the kinds the arm says it holds an entry of and
        // none of (docs/adr/0065 §6 R6b), viewed and held as its list is.
        let chained = |id: &str, arm: &GroupArm| {
            let field = fields
                .iter()
                .find(|f| f.id == id)
                .expect("a group's chain is a field of the codec");
            let Value::List { codec, bound, .. } = plan.value(field) else {
                unreachable!("the chain whose memberships are gates is a list")
            };
            let (class, namespace) = held_codec(imports, codec);
            let bounds = bounds_args(&bound);
            let (held_keys, lacked_keys) = group.kinds_in(arm);
            let key = |k: &u64| super::rust_source::key_type(struct_name, id, *k);
            let holds: Vec<String> = held_keys.iter().map(key).collect();
            let lacks: Vec<String> = lacked_keys.iter().map(key).collect();
            let kinds = |keys: &[String]| format!("::SCE::Forge::Kinds<{}>", keys.join(", "));
            let suffix: String = held_keys
                .iter()
                .map(|k| format!("_has_0x{k:x}"))
                .chain(lacked_keys.iter().map(|k| format!("_not_0x{k:x}")))
                .collect();
            let mut p = presented(id);
            let most = bound.most.map_or_else(
                || "::SCE::Forge::kUnbounded".to_string(),
                |most| most.to_string(),
            );
            // Where the one gate on a bare entry holds, the arm holds the
            // other entries and the named entry's place (R6a).
            let placed = group.member_is_bare() && !holds.is_empty();
            let (view, held, trait_name) = if placed {
                (
                    format!(
                        "::SCE::Forge::Placed<::SCE::Forge::Span<const {class}>, {}, {most}>",
                        holds[0]
                    ),
                    format!(
                        "::SCE::Forge::Placed<std::vector<{class}>, {}, {most}>",
                        holds[0]
                    ),
                    format!("is_{}_placed{suffix}", p.name),
                )
            } else {
                (
                    format!(
                        "::SCE::Forge::Chain<::SCE::Forge::Span<const {class}>, {}, {}, {bounds}>",
                        kinds(&holds),
                        kinds(&lacks)
                    ),
                    format!(
                        "::SCE::Forge::Chain<std::vector<{class}>, {}, {}, {bounds}>",
                        kinds(&holds),
                        kinds(&lacks)
                    ),
                    format!("is_{}_chain{suffix}", p.name),
                )
            };
            // An arm that holds no kind of a chain gated itself holds the
            // chain optionally (docs/adr/0065 §6 R6b).
            let optional = super::rust_source::chain_optional_in(group, field, arm);
            (p.view, p.held) = if optional {
                (
                    format!("std::optional<{view}>"),
                    format!("std::optional<{held}>"),
                )
            } else {
                (view.clone(), held)
            };
            p.chain = Some(ChainKinds {
                view,
                optional,
                placed,
                trait_name,
                holds,
                lacks,
                most,
                entry: class.to_string(),
                least: bound.least,
                namespace: namespace.to_string(),
            });
            p
        };
        let arms = group
            .arms
            .iter()
            .map(|arm| {
                let members = arm
                    .fields
                    .iter()
                    .filter(|id| !group.is_second_in(arm, id))
                    .map(|id| match group.second_of(arm, id) {
                        Some(second) => aligned(id, second),
                        None if group.chain() == Some(id.as_str()) => chained(id, arm),
                        None => presented(id),
                    })
                    .collect();
                if group.is_sum() {
                    let arm_name = super::rust_source::group_arm_name(group, arm);
                    ArmPart {
                        arm,
                        class: format!("{class}{arm_name}"),
                        members_trait: format!(
                            "is_{name}_{}_members",
                            filters::to_snake_case(arm_name)
                        ),
                        members,
                    }
                } else {
                    ArmPart {
                        arm,
                        class: class.clone(),
                        members_trait: format!("is_{name}_members"),
                        members,
                    }
                }
            })
            .collect();
        GroupPart {
            group,
            name,
            class,
            arms,
            members,
            at,
            struct_name: struct_name.to_string(),
        }
    }

    /// The type of the arm `index` of what `held`, the group as a source
    /// presents it or the value holds it where it is there, holds.
    fn arm_type(&self, held: &str, index: usize) -> String {
        if self.group.is_sum() {
            format!("std::variant_alternative_t<{index}, {held}>")
        } else {
            held.to_string()
        }
    }

    /// Each member of an arm that is two lists of one count, with the arm's
    /// index, in the arms' order.
    fn aligned(&self) -> impl Iterator<Item = (usize, &Presented<'p>, &AlignedLists)> {
        self.arms.iter().enumerate().flat_map(|(index, arm)| {
            arm.members
                .iter()
                .filter_map(move |m| m.aligned.as_ref().map(|lists| (index, m, lists)))
        })
    }

    /// What the encode's read of `member`, a body, a list or two lists of one
    /// count, returns, which it keeps (`_r_<member>`) as one type in every arm
    /// that holds it: its read off the first arm that holds it as a member of
    /// its own, or else, for a list an arm holds with the list it shares its
    /// count with, that list as the two the encode kept present it.
    fn kept_read(&self, held: &str, member: &str) -> String {
        if let Some(index) = self
            .arms
            .iter()
            .position(|arm| arm.members.iter().any(|a| a.name == member))
        {
            let read = format!(
                "decltype(source::{member}(std::declval<const {}&>()))",
                self.arm_type(held, index)
            );
            // A chain each arm holds as a type of its own kinds is kept as the
            // list it holds, one type in every arm.
            return if self.holds_chain(member) {
                format!("::SCE::Forge::Detail::InnerRead<{read}>")
            } else {
                read
            };
        }
        let (index, pair, side) = self
            .aligned()
            .find_map(|(index, m, lists)| {
                if lists.first == member {
                    Some((index, &m.name, "first"))
                } else if lists.second == member {
                    Some((index, &m.name, "second"))
                } else {
                    None
                }
            })
            .expect("a member of a group is a member of an arm");
        format!(
            "decltype(std::declval<const std::decay_t<decltype(source::{pair}(std::declval<const {}&>()))>&>().{side}())",
            self.arm_type(held, index)
        )
    }

    /// Whether `member` is the chain whose memberships are gates of the group,
    /// which each arm holds as a `Chain` of its own kinds.
    fn holds_chain(&self, member: &str) -> bool {
        self.arms
            .iter()
            .flat_map(|arm| arm.members.iter())
            .any(|m| m.name == member && m.chain.is_some())
    }

    /// Whether some arm holds `member` as a bare entry's place (docs/adr/0065
    /// §6 R6a), the others and where the named entry stands.
    fn holds_placed(&self, member: &str) -> bool {
        self.arms
            .iter()
            .flat_map(|arm| arm.members.iter())
            .any(|m| m.name == member && m.chain.as_ref().is_some_and(|c| c.placed))
    }

    /// The test, on the chain the decode read, that it holds an entry of the
    /// kind `key`.
    fn decoded_has(&self, l: &LangCtx, chain: &str, key: u64) -> String {
        format!(
            "::SCE::Forge::Detail::first_of_kind<{}>({}).has_value()",
            super::rust_source::key_type(&self.struct_name, chain, key),
            l.codec_field_id(chain)
        )
    }

    /// Whether a value may hold no arm of it, so that it holds it as a
    /// `std::optional`.
    fn optional(&self) -> bool {
        self.group.absent.is_some()
    }

    /// Whether an arm of it holds a byte string, a text, a body or a list,
    /// which it copies or moves in, so that it is made as a value holding
    /// bytes is: neither a constant expression nor free of throwing, and moved
    /// where it is handed on.
    fn holds_bytes(&self) -> bool {
        self.members.iter().any(Presented::owns)
    }

    /// What the value holds it as, and presents it as.
    fn holds(&self) -> String {
        if self.optional() {
            format!("std::optional<{}>", self.class)
        } else {
            self.class.clone()
        }
    }

    /// What a source's read of it is: `R` itself, or the type an optional `R`
    /// holds where it is there.
    fn held_of(&self, read: &str) -> String {
        if self.optional() {
            format!("typename std::decay_t<{read}>::value_type")
        } else {
            format!("std::decay_t<{read}>")
        }
    }

    /// The test, on the gates' locals of the decode, that the gates hold the
    /// values of `arm`: a flag input by its parameter, a one-bit flag of the
    /// codec's own by its carrier.
    fn decode_test(&self, arm: &GroupArm, model: &CodecModel, l: &LangCtx) -> String {
        let tests: Vec<String> = self
            .group
            .gates
            .iter()
            .zip(&arm.values)
            .map(|(gate, value)| {
                let cmp = if *value { "!=" } else { "==" };
                match gate {
                    GateVar::Input(input) => format!(
                        "(({} & 0x01) {cmp} 0)",
                        filters::to_snake_case(input.clone())
                    ),
                    GateVar::Flag { carrier, flag } => {
                        let bit = model
                            .fields
                            .iter()
                            .find(|f| f.id == *carrier)
                            .and_then(|f| f.flags.iter().find(|fl| fl.name == *flag))
                            .map(|fl| fl.bit)
                            .expect("a gate's flag is a flag of the codec's carrier");
                        format!("((({} >> {bit}) & 0x1) {cmp} 0)", l.codec_field_id(carrier))
                    }
                    // A membership holds where the chain the decode read holds an
                    // entry of the kind.
                    GateVar::Member { chain, key, .. } => {
                        let has = self.decoded_has(l, chain, *key);
                        if *value {
                            has
                        } else {
                            format!("!{has}")
                        }
                    }
                }
            })
            .collect();
        tests.join(" && ")
    }

    /// The gates of the group, as the document names them.
    fn label(&self) -> String {
        self.group
            .gates
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" || ")
    }

    /// The value of the gate `gate` of the group, where `held` is the group as
    /// the value holds it or a source presents it: for a group of one side,
    /// the side its arm gives it while the arm is there and the other side
    /// while it is not; for a sum, the value the arm there gives it, or the
    /// one the gates hold while no arm is.
    fn gate_value(&self, gate: &GateVar, held: &str) -> String {
        let index = self
            .group
            .gates
            .iter()
            .position(|g| g == gate)
            .expect("a gate of the group decides it");
        if let [arm] = self.group.arms.as_slice() {
            let (set, clear) = if arm.values[index] { (1, 0) } else { (0, 1) };
            return format!("static_cast<std::uint8_t>({held}.has_value() ? {set} : {clear})");
        }
        let which = if self.optional() {
            format!("{held}->index()")
        } else {
            format!("{held}.index()")
        };
        let set: Vec<String> = self
            .group
            .arms
            .iter()
            .enumerate()
            .filter(|(_, arm)| arm.values[index])
            .map(|(i, _)| format!("{which} == {i}u"))
            .collect();
        let in_arm = if set.is_empty() {
            "0".to_string()
        } else {
            format!("({}) ? 1 : 0", set.join(" || "))
        };
        match &self.group.absent {
            Some(absent) => format!(
                "static_cast<std::uint8_t>(!{held}.has_value() ? {} : ({in_arm}))",
                u8::from(absent[index])
            ),
            None => format!("static_cast<std::uint8_t>({in_arm})"),
        }
    }

    /// When the value holds `arm`, in words: each gate and whether it holds.
    fn condition(&self, arm: &GroupArm) -> String {
        self.group
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
    }

    /// The classes of its arms, each made by its constructor and read by its
    /// accessors as the codec's value is (ADR 0059 §5), and for a sum the
    /// variant of them.
    fn class_text(&self) -> String {
        let label = self.label();
        if let [arm] = self.arms.as_slice() {
            let side = if arm.arm.values.iter().all(|v| *v) {
                "holds"
            } else {
                "does not hold"
            };
            return arm.class_text(&format!(
                "/// The fields `{label}` decides (docs/adr/0065 §6 R4b), present together while\n\
                 /// `{label}` {side}: a value holds all of them or none, and its encode derives the\n\
                 /// gate from which.\n"
            ));
        }
        let mut out = String::new();
        for arm in &self.arms {
            out.push_str(&arm.class_text(&format!(
                "/// The fields `{label}` decides (docs/adr/0065 §6 R4b, R4c) present while {}.\n",
                self.condition(arm.arm)
            )));
        }
        let none = if self.optional() {
            " or none of them"
        } else {
            ""
        };
        out.push_str(&format!(
            "/// The fields `{label}` decides (docs/adr/0065 §6 R4b, R4c): a value holds one\n\
             /// arm{none}, and its encode derives the gates from which.\n\
             using {} = std::variant<{}>;\n\n",
            self.class,
            self.arms
                .iter()
                .map(|arm| arm.class.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        out
    }

    /// The traits, of the codec's namespace, that say whether a type presents
    /// each member of an arm, and whether what a source presents for the group
    /// is the shape the value holds it in of such types: a `std::optional` of
    /// one for a group of one side, a `std::variant` of one for each arm, in
    /// the arms' order, for a sum.
    fn traits(&self) -> String {
        let name = &self.name;
        let label = self.label();
        // What a list among the members is, wherever an arm presents it.
        let mut out: String = self
            .members
            .iter()
            .filter_map(Presented::entries_trait)
            .collect();
        // What two lists of one count are, wherever an arm presents them.
        let mut pairs: Vec<&str> = Vec::new();
        for (_, m, lists) in self.aligned() {
            if pairs.contains(&m.name.as_str()) {
                continue;
            }
            pairs.push(m.name.as_str());
            out.push_str(&format!(
                "/// Whether `R` is what a source may present for `{pair}`, two lists of one\n\
                 /// count (docs/adr/0065 §6 R5): an `Aligned` of a list `is_{first}_list`\n\
                 /// accepts and one `is_{second}_list` accepts.\n\
                 template <class R>\n\
                 struct is_{pair}_aligned : std::false_type {{}};\n\
                 template <class A, class B>\n\
                 struct is_{pair}_aligned<::SCE::Forge::Aligned<A, B>>\n\
                 \x20   : std::conjunction<is_{first}_list<A>, is_{second}_list<B>> {{}};\n\n",
                pair = m.name,
                first = lists.first,
                second = lists.second
            ));
        }
        // What the chain is in each arm, of the kinds the arm says of it.
        let mut chains: Vec<&str> = Vec::new();
        for m in self.arms.iter().flat_map(|arm| arm.members.iter()) {
            let Some(chain) = &m.chain else { continue };
            if chains.contains(&chain.trait_name.as_str()) {
                continue;
            }
            chains.push(chain.trait_name.as_str());
            if chain.placed {
                out.push_str(&format!(
                    "/// Whether `R` is what a source may present for `{member}` where the one gate\n\
                     /// on a bare entry holds (docs/adr/0065 §6 R6a): a `Placed` of the entry's\n\
                     /// kind, held to a bound within the chain's, of any list of the others,\n\
                     /// each a source of the entry codec.\n\
                     template <class R>\n\
                     struct {trait_name} : std::false_type {{}};\n\
                     template <class L, std::size_t Max>\n\
                     struct {trait_name}<::SCE::Forge::Placed<L, {key}, Max>>\n\
                     \x20   : std::conjunction<std::bool_constant<Max <= {most}>,\n\
                     \x20                      {ns}::is_source<::SCE::Forge::Detail::ElementOf<L>>> {{}};\n\n",
                    member = m.name,
                    trait_name = chain.trait_name,
                    key = chain.holds[0],
                    most = chain.most,
                    ns = chain.namespace
                ));
                continue;
            }
            out.push_str(&format!(
                "/// Whether `R` is what a source may present for `{member}` where the chain holds\n\
                 /// an entry of each kind of `{holds}` and of none of `{lacks}`\n\
                 /// (docs/adr/0065 §6 R6b): a `Chain` of those kinds, held to bounds within its\n\
                 /// own, of any list, each entry a source of the entry codec.\n\
                 template <class R>\n\
                 struct {trait_name} : std::false_type {{}};\n\
                 template <class L, std::size_t Max, std::size_t Min>\n\
                 struct {trait_name}<::SCE::Forge::Chain<L, {holds}, {lacks}, Max, Min>>\n\
                 \x20   : std::conjunction<::SCE::Forge::Detail::WithinBounds<{most}, {least}, Max, Min>,\n\
                 \x20                      {ns}::is_source<::SCE::Forge::Detail::ElementOf<L>>> {{}};\n\n",
                member = m.name,
                trait_name = chain.trait_name,
                holds = format!("::SCE::Forge::Kinds<{}>", chain.holds.join(", ")),
                lacks = format!("::SCE::Forge::Kinds<{}>", chain.lacks.join(", ")),
                most = chain.most,
                least = chain.least,
                ns = chain.namespace
            ));
            if chain.optional {
                out.push_str(&format!(
                    "/// Whether `R` is what a source may present for `{member}` where the arm may\n\
                     /// hold no chain: a `std::optional` of a type `{trait_name}` accepts.\n\
                     template <class R>\n\
                     struct {present} : std::false_type {{}};\n\
                     template <class C>\n\
                     struct {present}<std::optional<C>> : {trait_name}<C> {{}};\n\n",
                    member = m.name,
                    trait_name = chain.trait_name,
                    present = chain.presented_trait()
                ));
            }
        }
        for arm in &self.arms {
            out.push_str(&arm.members_traits());
        }
        if let [arm] = self.arms.as_slice() {
            out.push_str(&format!(
                "/// Whether `R` is what a source may present for `{name}`, the fields `{label}`\n\
                 /// decides: a `std::optional` of a type `{trait_name}` accepts.\n\
                 template <class R>\n\
                 struct is_{name}_group : std::false_type {{}};\n\
                 template <class G>\n\
                 struct is_{name}_group<std::optional<G>> : {trait_name}<G> {{}};\n\n",
                trait_name = arm.members_trait
            ));
            return out;
        }
        let params: Vec<String> = (0..self.arms.len())
            .map(|i| format!("class G{i}"))
            .collect();
        let args: Vec<String> = (0..self.arms.len()).map(|i| format!("G{i}")).collect();
        let mut accepts: Vec<String> = self
            .arms
            .iter()
            .enumerate()
            .map(|(i, arm)| format!("{}<G{i}>", arm.members_trait))
            .collect();
        // A body or a list the encode keeps is kept as one type, so every arm
        // that holds it presents it as the type the first one does, as the Rust
        // group holds it as one type parameter in every arm.
        // Two lists of one count that more than one arm holds likewise.
        let kept_names = self
            .members
            .iter()
            .filter(|m| m.kept_as_read())
            .map(|m| m.name.as_str())
            .chain(pairs.iter().copied());
        let mut kept_alike: Vec<String> = Vec::new();
        for name in kept_names {
            let holding: Vec<usize> = self
                .arms
                .iter()
                .enumerate()
                .filter(|(_, arm)| arm.members.iter().any(|a| a.name == name))
                .map(|(i, _)| i)
                .collect();
            // A list one arm holds as a member of its own and another holds
            // with the list it shares its count with is kept as one type too:
            // the type the first arm presents it as, which the list the other
            // arm's two present converts to.
            if let Some(first) = holding.first() {
                for (index, pair, lists) in self.aligned() {
                    let side = if lists.first == name {
                        "first"
                    } else if lists.second == name {
                        "second"
                    } else {
                        continue;
                    };
                    let alike = format!("is_{name}_kept_alike_in_{}", pair.name);
                    let plain = format!("decltype(source::{name}(std::declval<const A&>()))");
                    let held = format!(
                        "decltype(std::declval<const std::decay_t<decltype(source::{}(std::declval<const B&>()))>&>().{side}())",
                        pair.name
                    );
                    if !kept_alike.contains(&alike) {
                        kept_alike.push(alike.clone());
                        out.push_str(&format!(
                            "/// Whether `A` presents `{name}` as the type the two lists of one count\n\
                             /// `B` presents hold it as, which converts to what `A` presents.\n\
                             template <class A, class B, class = void>\n\
                             struct {alike} : std::false_type {{}};\n\
                             template <class A, class B>\n\
                             struct {alike}<A, B, std::void_t<{plain}, {held}>>\n\
                             \x20   : std::conjunction<std::is_same<std::decay_t<{plain}>, std::decay_t<{held}>>,\n\
                             \x20                      std::is_convertible<{held}, {plain}>> {{}};\n\n"
                        ));
                    }
                    accepts.push(format!("{alike}<G{first}, G{index}>"));
                }
            }
            if let [first, rest @ ..] = holding.as_slice() {
                if !rest.is_empty() && self.holds_chain(name) {
                    // A chain each arm holds as a type of its own kinds is kept
                    // as the list it holds: every arm presents that one list.
                    let member = name;
                    let read = |ty: &str| {
                        format!("decltype(source::{member}(std::declval<const {ty}&>()))")
                    };
                    let inner = |ty: &str| format!("::SCE::Forge::Detail::InnerRead<{}>", read(ty));
                    out.push_str(&format!(
                        "/// Whether `A` and `B` both present `{member}` as a chain of one list, kept\n\
                         /// alike.\n\
                         template <class A, class B, class = void>\n\
                         struct is_{member}_read_alike : std::false_type {{}};\n\
                         template <class A, class B>\n\
                         struct is_{member}_read_alike<A, B, std::void_t<{}, {}>>\n\
                         \x20   : std::is_same<{}, {}> {{}};\n\n",
                        inner("A"),
                        inner("B"),
                        inner("A"),
                        inner("B")
                    ));
                } else if !rest.is_empty() {
                    let member = name;
                    let read = |ty: &str| {
                        format!("decltype(source::{member}(std::declval<const {ty}&>()))")
                    };
                    out.push_str(&format!(
                        "/// Whether `A` and `B` both present `{member}`, as one type.\n\
                         template <class A, class B, class = void>\n\
                         struct is_{member}_read_alike : std::false_type {{}};\n\
                         template <class A, class B>\n\
                         struct is_{member}_read_alike<A, B, std::void_t<{}, {}>>\n\
                         \x20   : std::is_same<{}, {}> {{}};\n\n",
                        read("A"),
                        read("B"),
                        read("A"),
                        read("B")
                    ));
                }
                for i in rest {
                    accepts.push(format!("is_{name}_read_alike<G{first}, G{i}>"));
                }
            }
        }
        let variant = format!("std::variant<{}>", args.join(", "));
        let (shape, of) = if self.optional() {
            (
                format!("std::optional<{variant}>"),
                "a `std::optional` of a `std::variant`",
            )
        } else {
            (variant, "a `std::variant`")
        };
        out.push_str(&format!(
            "/// Whether `R` is what a source may present for `{name}`, the fields `{label}`\n\
             /// decides: {of} of a type for each arm, in order, that presents its fields.\n\
             template <class R>\n\
             struct is_{name}_group : std::false_type {{}};\n\
             template <{}>\n\
             struct is_{name}_group<{shape}>\n\
             \x20   : std::conjunction<{}> {{}};\n\n",
            params.join(", "),
            accepts.join(", ")
        ));
        out
    }

    /// What a source's read of it must be.
    fn source_requirement(&self) -> String {
        format!(
            "is_{name}_group<std::decay_t<decltype(source::{name}(std::declval<const S&>()))>>",
            name = self.name
        )
    }

    /// Whether reading it from a source `S`, and each member of each arm off
    /// what that read presents for the arm, throws nothing.
    fn nothrow_requirement(&self) -> String {
        let read = format!("source::{}(std::declval<const S&>())", self.name);
        let held = self.held_of(&format!("decltype({read})"));
        let mut parts = vec![format!("noexcept({read})")];
        let sum = self.group.is_sum();
        for (i, arm) in self.arms.iter().enumerate() {
            let presents = if sum {
                format!("std::variant_alternative_t<{i}, {held}>")
            } else {
                held.clone()
            };
            for m in &arm.members {
                let read = format!("source::{}(std::declval<const {presents}&>())", m.name);
                parts.push(m.member_nothrow(&read));
                // Each of two lists of one count is kept off what the encode
                // kept of the two, as the list it is in every arm.
                if let Some(lists) = &m.aligned {
                    for (list, side) in [(&lists.first, "first"), (&lists.second, "second")] {
                        parts.push(format!(
                            "noexcept(std::declval<::SCE::Forge::Detail::Kept<{}>&>().keep(std::declval<const std::decay_t<decltype({read})>&>().{side}()))",
                            self.kept_read(&held, list)
                        ));
                    }
                }
            }
        }
        parts.join(" && ")
    }

    /// The encode's binding of each member to its local from the arm the value
    /// holds, where it holds one that holds the member: an optional value of
    /// its type (`_e_<member>`), or, for a body or a list, the read kept as
    /// the arms that hold it return it, one type in all of them (`_r_<member>`,
    /// a `::SCE::Forge::Detail::Kept`).
    fn bind_members(&self) -> String {
        let name = &self.name;
        let mut out = format!("        const auto& _r_{name} = source::{name}(src);\n");
        let held = self.held_of(&format!("decltype(_r_{name})"));
        for m in &self.members {
            if m.kept_as_read() {
                out.push_str(&format!(
                    "        ::SCE::Forge::Detail::Kept<{}> _r_{};\n",
                    self.kept_read(&held, &m.name),
                    m.name
                ));
                // Where the named entry of a bare entry's chain stands among
                // the others, where the arm the value holds holds it.
                if self.holds_placed(&m.name) {
                    out.push_str(&format!(
                        "        std::optional<std::size_t> _p_{};\n",
                        m.name
                    ));
                }
            } else {
                out.push_str(&format!(
                    "        std::optional<{}> _e_{};\n",
                    m.view, m.name
                ));
            }
        }
        // Two lists of one count are kept as the arm presents them, and each
        // list read off what is kept, which lives as long as the encode.
        let mut pairs: Vec<&str> = Vec::new();
        for (_, m, _) in self.aligned() {
            if !pairs.contains(&m.name.as_str()) {
                pairs.push(m.name.as_str());
                out.push_str(&format!(
                    "        ::SCE::Forge::Detail::Kept<{}> _r_{};\n",
                    self.kept_read(&held, &m.name),
                    m.name
                ));
            }
        }
        let bind = |indent: &str, from: &str, members: &[Presented<'_>]| {
            members
                .iter()
                .map(|m| {
                    if let Some(lists) = &m.aligned {
                        format!(
                            "{indent}_r_{pair}.keep(source::{pair}({from}));\n\
                             {indent}_r_{first}.keep(_r_{pair}->first());\n\
                             {indent}_r_{second}.keep(_r_{pair}->second());\n",
                            pair = m.name,
                            first = lists.first,
                            second = lists.second
                        )
                    } else if let Some(chain) = &m.chain {
                        // A chain the arm may hold none of is kept where it is
                        // there; the read it is bound to lives through the keep.
                        if chain.optional {
                            format!(
                                "{indent}if (const auto& _c_{member} = source::{member}({from})) {{\n\
                                 {indent}    _r_{member}.keep(_c_{member}->inner());\n\
                                 {indent}}}\n",
                                member = m.name
                            )
                        } else if chain.placed {
                            // A bare entry's place keeps the others and where
                            // the named entry stands among them.
                            format!(
                                "{indent}{{\n\
                                 {indent}    const auto& _c_{member} = source::{member}({from});\n\
                                 {indent}    _r_{member}.keep(_c_{member}.rest());\n\
                                 {indent}    _p_{member} = _c_{member}.place();\n\
                                 {indent}}}\n",
                                member = m.name
                            )
                        } else {
                            format!(
                                "{indent}_r_{member}.keep(source::{member}({from}).inner());\n",
                                member = m.name
                            )
                        }
                    } else if m.kept_as_read() {
                        format!(
                            "{indent}_r_{member}.keep(source::{member}({from}));\n",
                            member = m.name
                        )
                    } else {
                        format!(
                            "{indent}_e_{member} = static_cast<{view}>(source::{member}({from}));\n",
                            member = m.name,
                            view = m.view
                        )
                    }
                })
                .collect::<String>()
        };
        if let [arm] = self.arms.as_slice() {
            out.push_str(&format!("        if (_r_{name}) {{\n"));
            out.push_str(&bind("            ", &format!("*_r_{name}"), &arm.members));
            out.push_str("        }\n");
            return out;
        }
        let (open, variant, indent, close) = if self.optional() {
            (
                format!("        if (_r_{name}) {{\n"),
                format!("&*_r_{name}"),
                "            ",
                "        }\n",
            )
        } else {
            (String::new(), format!("&_r_{name}"), "        ", "")
        };
        out.push_str(&open);
        for (i, arm) in self.arms.iter().enumerate() {
            if arm.members.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "{indent}if (const auto* _a = std::get_if<{i}>({variant})) {{\n"
            ));
            out.push_str(&bind(&format!("{indent}    "), "*_a", &arm.members));
            out.push_str(&format!("{indent}}}\n"));
        }
        out.push_str(close);
        out
    }

    /// The decode's making of the group from the members it read: for a
    /// group of one side, its arm where every member is there; for a sum, the
    /// arm whose values the gates the decode read hold. A byte string, a text
    /// or a list is taken as its bound's view of what was read (`_b_<member>`),
    /// and a body the decode read is moved in; a member the decode reads
    /// whatever the gate holds, such as a length the side without its bytes
    /// states, as it was read.
    fn decoded(&self, model: &CodecModel, l: &LangCtx) -> String {
        let (holds, name) = (self.holds(), &self.name);
        // Two lists of one count are made of the two lists the decode read by
        // the one count, which are then of one count: a frame that read them
        // apart would have held more entries in one of them than the count.
        let pairs_made = |arm: &ArmPart<'_>, indent: &str| {
            arm.members
                .iter()
                .filter_map(|m| m.aligned.as_ref().map(|lists| (m, lists)))
                .map(|(m, lists)| {
                    format!(
                        "{indent}const auto _al_{pair} = {view}::make(*_b_{first}, *_b_{second});\n\
                         {indent}if (!_al_{pair}) {{\n\
                         {indent}    cursor.refuse(::SCE::Forge::CodecError::TooManyElements);\n\
                         {indent}    return std::nullopt;\n\
                         {indent}}}\n",
                        pair = m.name,
                        view = m.view,
                        first = lists.first,
                        second = lists.second
                    )
                })
                .chain(
                    arm.members
                        .iter()
                        .filter_map(|m| m.chain.as_ref().map(|chain| (m, chain)))
                        .map(|(m, chain)| {
                            // The chain the decode read, which holds the kinds the
                            // gates it chose the arm by test: the arm's chain, as
                            // its type says; where the arm may hold none, the
                            // chain where the decode read one.
                            if chain.placed {
                                // A bare entry's place: the first entry of the
                                // kind, which the gate the arm was chosen by
                                // found, is the named one, and the others are
                                // held apart from it.
                                format!(
                                    "{indent}std::vector<{entry}> _rest_{name}(_b_{name}->inner().begin(), _b_{name}->inner().end());\n\
                                     {indent}const auto _at_{name} = ::SCE::Forge::Detail::first_of_kind<{key}>(_rest_{name});\n\
                                     {indent}if (!_at_{name}) {{\n\
                                     {indent}    cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);\n\
                                     {indent}    return std::nullopt;\n\
                                     {indent}}}\n\
                                     {indent}_rest_{name}.erase(_rest_{name}.begin() + static_cast<std::ptrdiff_t>(*_at_{name}));\n\
                                     {indent}const auto _ch_{name} = {view}::make(::SCE::Forge::Span<const {entry}>(_rest_{name}.data(), _rest_{name}.size()), *_at_{name});\n\
                                     {indent}if (!_ch_{name}) {{\n\
                                     {indent}    cursor.refuse(::SCE::Forge::CodecError::TooManyElements);\n\
                                     {indent}    return std::nullopt;\n\
                                     {indent}}}\n",
                                    name = m.name,
                                    view = chain.view,
                                    key = chain.holds[0],
                                    entry = chain.entry
                                )
                            } else if chain.optional {
                                format!(
                                    "{indent}std::optional<{view}> _ch_{name};\n\
                                     {indent}if (_b_{name}) {{\n\
                                     {indent}    _ch_{name} = {view}::make(_b_{name}->inner());\n\
                                     {indent}    if (!_ch_{name}) {{\n\
                                     {indent}        cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);\n\
                                     {indent}        return std::nullopt;\n\
                                     {indent}    }}\n\
                                     {indent}}}\n",
                                    name = m.name,
                                    view = chain.view
                                )
                            } else {
                                format!(
                                    "{indent}const auto _ch_{name} = {view}::make(_b_{name}->inner());\n\
                                     {indent}if (!_ch_{name}) {{\n\
                                     {indent}    cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);\n\
                                     {indent}    return std::nullopt;\n\
                                     {indent}}}\n",
                                    name = m.name,
                                    view = chain.view
                                )
                            }
                        }),
                )
                .collect::<String>()
        };
        // A member read optionally is there where its value is, and two lists
        // of one count where both are.
        let read_optionally = |list: &str| {
            self.members
                .iter()
                .any(|m| m.name == list && m.read_optionally())
        };
        let args = |arm: &ArmPart<'_>| {
            arm.members
                .iter()
                .map(|m| {
                    if m.aligned.is_some() {
                        return format!("*_al_{}", m.name);
                    }
                    if let Some(chain) = &m.chain {
                        return if chain.optional {
                            format!("_ch_{}", m.name)
                        } else {
                            format!("*_ch_{}", m.name)
                        };
                    }
                    let read = if m.read_optionally() {
                        format!("*{}", m.name)
                    } else {
                        m.name.clone()
                    };
                    if m.bytes || m.list.is_some() {
                        format!("*_b_{}", m.name)
                    } else if m.body.is_some() {
                        format!("std::move({read})")
                    } else {
                        read
                    }
                })
                .collect::<Vec<_>>()
        };
        if let [arm] = self.arms.as_slice() {
            let there: Vec<String> = arm
                .members
                .iter()
                .flat_map(|m| match &m.aligned {
                    Some(lists) => vec![lists.first.as_str(), lists.second.as_str()],
                    None => vec![m.name.as_str()],
                })
                .filter(|list| read_optionally(list))
                .map(|list| format!("{list}.has_value()"))
                .collect();
            return format!(
                "        {holds} _g_{name};\n\
                 \x20       if ({there}) {{\n\
                 {pairs}\
                 \x20           _g_{name}.emplace({args});\n\
                 \x20       }}\n",
                there = there.join(" && "),
                pairs = pairs_made(arm, "            "),
                args = args(arm).join(", ")
            );
        }
        // A group with an arm that has no value made by default starts as the
        // arm a value made by default holds, which has one, and is then made
        // the arm the gates hold.
        let made_by_default = self.arms.iter().all(|arm| {
            !arm.members
                .iter()
                .any(|m| m.chain.as_ref().is_some_and(|c| !c.has_default()))
        });
        let mut out = if made_by_default {
            format!("        {holds} _g_{name};\n")
        } else {
            format!("        {holds} _g_{name}{};\n", self.default_init())
        };
        for (i, arm) in self.arms.iter().enumerate() {
            let keyword = if i == 0 { "if" } else { "} else if" };
            let made = if self.optional() {
                let mut all = vec![format!("std::in_place_index<{i}>")];
                all.extend(args(arm));
                format!("_g_{name}.emplace({})", all.join(", "))
            } else {
                format!("_g_{name}.emplace<{i}>({})", args(arm).join(", "))
            };
            out.push_str(&format!(
                "        {keyword} ({}) {{\n\
                 {}\
                 \x20           {made};\n",
                self.decode_test(arm.arm, model, l),
                pairs_made(arm, "            ")
            ));
        }
        out.push_str("        }\n");
        out
    }

    /// The initializer of the group a value made by default holds: none of a
    /// group the value may hold none of, and otherwise the arm present while
    /// no gate holds, where there is one, else the first.
    fn default_init(&self) -> String {
        if self.optional() {
            return "{}".to_string();
        }
        match self
            .group
            .arms
            .iter()
            .position(|arm| arm.values.iter().all(|v| !*v))
        {
            Some(index) if index > 0 => format!("{{std::in_place_index_t<{index}>{{}}}}"),
            _ => "{}".to_string(),
        }
    }
}

impl ArmPart<'_> {
    /// Its class, under the doc comment `doc`.
    fn class_text(&self, doc: &str) -> String {
        let class = &self.class;
        let params: Vec<String> = self
            .members
            .iter()
            .map(|m| format!("{} {}", m.view, m.name))
            .collect();
        // A body is moved in, as the value moves one in.
        let inits: Vec<String> = self
            .members
            .iter()
            .map(|m| {
                if m.body.is_some() {
                    format!("{}(std::move({}))", member(&m.name), m.name)
                } else {
                    format!("{}({})", member(&m.name), m.name)
                }
            })
            .collect();
        let mut out = format!("{doc}class {class} {{\npublic:\n");
        // An arm whose chain holds an entry of some kind has no value made by
        // default: no list of value-made entries holds one. A bare entry's
        // place has one, the named entry alone.
        if !self
            .members
            .iter()
            .any(|m| m.chain.as_ref().is_some_and(|c| !c.has_default()))
        {
            out.push_str(&format!("    {class}() = default;\n"));
        }
        // An arm that holds a byte string, a text, a body or a list copies or
        // moves it in, which may throw and is no constant expression in C++17,
        // as for the value.
        let (constexpr, noexcept) = if self.members.iter().any(Presented::owns) {
            ("", "")
        } else {
            ("constexpr ", " noexcept")
        };
        if !self.members.is_empty() {
            out.push_str(&format!(
                "    /// The value of each field, in declaration order.\n\
                 \x20   {constexpr}{class}({}){noexcept}\n\
                 \x20       : {} {{}}\n",
                params.join(", "),
                inits.join(", ")
            ));
        }
        // A body is presented by reference, as the value presents one; a list
        // or a byte string, as its bound's view of what the arm holds.
        for m in &self.members {
            let constexpr = if m.owns() { "" } else { "constexpr " };
            let view = if m.body.is_some() {
                format!("const {}&", m.view)
            } else {
                m.view.clone()
            };
            out.push_str(&format!(
                "    /// The `{name}` field.\n\
                 \x20   [[nodiscard]] {constexpr}{view} {name}() const noexcept {{ return {held}; }}\n",
                name = m.name,
                held = member(&m.name)
            ));
        }
        if !self.members.is_empty() {
            out.push_str("\nprivate:\n");
            for m in &self.members {
                // A chain that holds an entry of some kind has no value made by
                // default, and is made by the arm's constructor alone; a bare
                // entry's place has one.
                let init = if m.chain.as_ref().is_some_and(|c| !c.has_default()) {
                    ""
                } else {
                    "{}"
                };
                out.push_str(&format!("    {} {}{init};\n", m.held, member(&m.name)));
            }
        }
        out.push_str("};\n\n");
        out
    }

    /// The trait that says whether a type presents each of its members, each
    /// as [`Presented::member_requirements`] asks.
    fn members_traits(&self) -> String {
        let read =
            |m: &Presented<'_>| format!("decltype(source::{}(std::declval<const G&>()))", m.name);
        let reads: Vec<String> = self.members.iter().map(read).collect();
        let converts: Vec<String> = self
            .members
            .iter()
            .flat_map(|m| m.member_requirements(&read(m)))
            .collect();
        let (class, trait_name) = (&self.class, &self.members_trait);
        if self.members.is_empty() {
            return format!(
                "/// Whether `G` presents every field of `{class}`, which has none.\n\
                 template <class G>\n\
                 struct {trait_name} : std::true_type {{}};\n"
            );
        }
        format!(
            "/// Whether `G` presents every field of `{class}`, each convertible to its type.\n\
             template <class G, class = void>\n\
             struct {trait_name} : std::false_type {{}};\n\
             template <class G>\n\
             struct {trait_name}<G, std::void_t<{}>>\n\
             \x20   : std::conjunction<{}> {{}};\n",
            reads.join(", "),
            converts.join(", ")
        )
    }
}

/// An element of a printed value, or a group of its elements, in the order its
/// fields are declared: the order of its constructor's parameters.
enum Part {
    Element(usize),
    Group(usize),
}

impl<'p> Presented<'p> {
    /// Whether the decode reads it into a `std::optional` local: a field of its
    /// own gate. A member of a group the decode reads whatever the gate holds,
    /// such as a length the side without its bytes states, is read as it is.
    fn read_optionally(&self) -> bool {
        self.field.present_if.is_some()
    }

    /// Whether it is held as what owns what it holds (a byte string, a text, a
    /// body or a list), copied or moved in, so that what holds it is made as a
    /// value holding bytes is: neither a constant expression nor free of
    /// throwing.
    fn owns(&self) -> bool {
        self.bytes || self.body.is_some() || self.list.is_some() || self.aligned.is_some()
    }

    /// Whether the encode keeps it as the source presents it, where it is a
    /// member of a group: a body or a list, which it reads through the codec
    /// it holds rather than converts, or two lists of one count, whose lists
    /// it reads off what it keeps.
    fn kept_as_read(&self) -> bool {
        self.body.is_some() || self.list.is_some() || self.aligned.is_some()
    }

    /// What it is presented as: its accessor's type, its constructor's
    /// parameter, and what a source's read of it converts to.
    fn presented(&self) -> String {
        if self.optional {
            format!("std::optional<{}>", self.view)
        } else {
            self.view.clone()
        }
    }

    /// What the value holds it as.
    fn holds(&self) -> String {
        if self.optional {
            format!("std::optional<{}>", self.held)
        } else {
            self.held.clone()
        }
    }

    /// Whether the encode reads it once, into `_e_<name>`, before it writes:
    /// a byte string is viewed within its bound, and an optional value is
    /// tested before it is written, so each is read once and kept.
    fn kept(&self) -> bool {
        self.bytes || self.optional
    }

    /// The local the encode holds it in once read: a body or a list as the
    /// source presents it (`_r_<name>`), anything else as what it is presented
    /// as (`_e_<name>`). For an optional element it is what is tested present.
    fn local(&self) -> String {
        if self.body.is_some() || self.list.is_some() {
            format!("_r_{}", self.name)
        } else {
            format!("_e_{}", self.name)
        }
    }

    /// The trait, of the codec's namespace, that says whether what a source
    /// presents for a body a gate decides is a `std::optional` of a source of
    /// the body's codec; `None` for an element that is no such body.
    fn body_trait(&self) -> Option<String> {
        let held = self.body.as_ref().filter(|_| self.optional)?;
        Some(format!(
            "/// Whether `R` is what a source may present for `{name}`, a body a gate\n\
             /// decides: a `std::optional` of a source of the body's codec.\n\
             template <class R>\n\
             struct is_{name}_body : std::false_type {{}};\n\
             template <class B>\n\
             struct is_{name}_body<std::optional<B>> : {ns}::is_source<B> {{}};\n\n",
            name = self.name,
            ns = held.namespace
        ))
    }

    /// `element` as the value presents it; `law`, for a list whose entries
    /// say inputs it hands them, the type of the law it holds them to.
    fn of(
        element: &crate::forge::encode_plan::Element<'p>,
        l: &LangCtx,
        imports: &[ImportContext],
        law: Option<String>,
    ) -> Self {
        let name = l.codec_field_id(&element.field.id);
        let optional = element.presence == Presence::Gated;
        match &element.value {
            Value::Body { codec } => {
                let (class, namespace) = held_codec(imports, codec);
                Presented {
                    field: element.field,
                    name,
                    view: class.to_string(),
                    held: class.to_string(),
                    bytes: false,
                    text: false,
                    optional,
                    body: Some(HeldCodec {
                        namespace: namespace.to_string(),
                        alias: (*codec).to_string(),
                    }),
                    list: None,
                    law: None,
                    aligned: None,
                    chain: None,
                }
            }
            Value::List { codec, bound, .. } => {
                let (class, namespace) = held_codec(imports, codec);
                let bounds = bounds_args(bound);
                let view = format!("::SCE::Forge::List<{class}, {bounds}>");
                let held = format!("::SCE::Forge::Bounded<std::vector<{class}>, {bounds}>");
                let (view, held) = match &law {
                    Some(law) => (
                        format!("::SCE::Forge::Lawful<{view}, {law}>"),
                        format!("::SCE::Forge::Lawful<{held}, {law}>"),
                    ),
                    None => (view, held),
                };
                Presented {
                    field: element.field,
                    name,
                    view,
                    held,
                    bytes: false,
                    text: false,
                    optional,
                    body: None,
                    list: Some((
                        HeldCodec {
                            namespace: namespace.to_string(),
                            alias: (*codec).to_string(),
                        },
                        bounds,
                    )),
                    law,
                    aligned: None,
                    chain: None,
                }
            }
            Value::Bytes(bound) | Value::Text(bound) => {
                let text = matches!(element.value, Value::Text(_));
                let bounds = bounds_args(bound);
                let (view, list) = if text {
                    ("Text", "std::string")
                } else {
                    ("Bytes", "std::vector<std::uint8_t>")
                };
                Presented {
                    field: element.field,
                    name,
                    view: format!("::SCE::Forge::{view}<{bounds}>"),
                    held: format!("::SCE::Forge::Bounded<{list}, {bounds}>"),
                    bytes: true,
                    text,
                    optional,
                    body: None,
                    list: None,
                    law: None,
                    aligned: None,
                    chain: None,
                }
            }
            _ => {
                let ty = l.type_name(&element.field.sce_type).to_string();
                Presented {
                    field: element.field,
                    name,
                    view: ty.clone(),
                    held: ty,
                    bytes: false,
                    text: false,
                    optional,
                    body: None,
                    list: None,
                    law: None,
                    aligned: None,
                    chain: None,
                }
            }
        }
    }

    /// The trait, of the codec's namespace, that says whether a list a source
    /// presents for this element lies within its bounds and holds sources of
    /// the entry codec, and for a list a gate decides the one that says it is
    /// a `std::optional` of such a list; `None` for an element that is no list.
    fn list_trait(&self) -> Option<String> {
        let mut out = self.entries_trait()?;
        if self.optional {
            out.push_str(&format!(
                "/// Whether `R` is what a source may present for `{name}`, a list a gate\n\
                 /// decides: a `std::optional` of a list `is_{name}_list` accepts.\n\
                 template <class R>\n\
                 struct is_{name}_present_list : std::false_type {{}};\n\
                 template <class L>\n\
                 struct is_{name}_present_list<std::optional<L>> : is_{name}_list<L> {{}};\n\n",
                name = self.name
            ));
        }
        Some(out)
    }

    /// The trait, of the codec's namespace, that says whether a list a source
    /// presents for this element where it is there (`is_<name>_list`) lies
    /// within its bounds, holds sources of the entry codec and, for a list held
    /// to a law, is held to it; `None` for an element that is no list.
    fn entries_trait(&self) -> Option<String> {
        let (held, bounds) = self.list.as_ref()?;
        let name = &self.name;
        // A list held to a law is a `Lawful` of that law over such a list.
        let entries = if self.law.is_some() {
            format!("is_{name}_bounded")
        } else {
            format!("is_{name}_list")
        };
        let mut out = format!(
            "/// Whether `R` is a list a source may present for `{name}`: held to bounds\n\
             /// within its own, of any type, each entry a source of the entry codec.\n\
             template <class R, class = void>\n\
             struct {entries} : std::false_type {{}};\n\
             template <class R>\n\
             struct {entries}<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, {bounds}>::value>>\n\
             \x20   : {ns}::is_source<::SCE::Forge::Detail::ElementOf<R>> {{}};\n\n",
            ns = held.namespace
        );
        if let Some(law) = &self.law {
            out.push_str(&format!(
                "/// Whether `R` is a list a source may present for `{name}`: a list\n\
                 /// `{entries}` accepts, held to `{law}` (docs/adr/0065 §6 R8).\n\
                 template <class R>\n\
                 struct is_{name}_list : std::false_type {{}};\n\
                 template <class L>\n\
                 struct is_{name}_list<::SCE::Forge::Lawful<L, {law}>> : {entries}<L> {{}};\n\n"
            ));
        }
        Some(out)
    }

    /// The entries of a list the encode read into its local, as a range: the
    /// list the value holds where a gate leaves it in, read only where it is
    /// tested present.
    fn entries(&self) -> String {
        if self.optional {
            format!("(*_r_{})", self.name)
        } else {
            format!("_r_{}", self.name)
        }
    }

    /// What a source's read of it must be: convertible to what it is
    /// presented as, or, for a body, a source of the codec it holds.
    fn source_requirement(&self) -> String {
        let read = format!("decltype(source::{}(std::declval<const S&>()))", self.name);
        match (&self.body, &self.list) {
            (Some(_), _) if self.optional => format!("is_{}_body<std::decay_t<{read}>>", self.name),
            (Some(held), _) => format!("{}::is_source<std::decay_t<{read}>>", held.namespace),
            (None, Some(_)) if self.optional => {
                format!("is_{}_present_list<std::decay_t<{read}>>", self.name)
            }
            (None, Some(_)) => format!("is_{}_list<std::decay_t<{read}>>", self.name),
            (None, None) => format!("std::is_convertible<{read}, {}>", self.presented()),
        }
    }

    /// Whether reading it from a source `S`, and taking it as what it is
    /// presented as, throws nothing; for a body, whether the read and every
    /// read of the body as a source of its codec throw nothing.
    fn nothrow_requirement(&self) -> String {
        let read = format!("source::{}(std::declval<const S&>())", self.name);
        match (&self.body, &self.list) {
            // An optional body's value is read as a source of the body's codec.
            (Some(held), _) if self.optional => format!(
                "noexcept({read}) && {}::is_nothrow_source<typename std::decay_t<decltype({read})>::value_type>::value",
                held.namespace
            ),
            (Some(held), _) => format!(
                "noexcept({read}) && {}::is_nothrow_source<std::decay_t<decltype({read})>>::value",
                held.namespace
            ),
            // A list's entries are read as sources of the entry codec; an
            // optional list's, off the list it holds.
            (None, Some((held, _))) if self.optional => format!(
                "noexcept({read}) && {}::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename std::decay_t<decltype({read})>::value_type>>::value",
                held.namespace
            ),
            (None, Some((held, _))) => format!(
                "noexcept({read}) && {}::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype({read})>>>::value",
                held.namespace
            ),
            (None, None) => format!(
                "noexcept(::SCE::Forge::Detail::converts_to<{}>({read}))",
                self.presented()
            ),
        }
    }

    /// What `read`, the type a read of it as a member of a group's arm
    /// returns, must be: a source of the codec of a body, a list
    /// `is_<name>_list` accepts, or what converts to its type. The encode keeps
    /// the view of a byte string or a text past the read, so that read must
    /// present what lives on (`::SCE::Forge::Detail::Lasting`); it keeps a body
    /// or a list as read (`::SCE::Forge::Detail::Kept`).
    fn member_requirements(&self, read: &str) -> Vec<String> {
        if self.aligned.is_some() {
            return vec![format!("is_{}_aligned<std::decay_t<{read}>>", self.name)];
        }
        if let Some(chain) = &self.chain {
            return vec![format!("{}<std::decay_t<{read}>>", chain.presented_trait())];
        }
        match (&self.body, &self.list) {
            (Some(held), _) => vec![format!(
                "{}::is_source<std::decay_t<{read}>>",
                held.namespace
            )],
            (None, Some(_)) => vec![format!("is_{}_list<std::decay_t<{read}>>", self.name)],
            (None, None) => {
                let mut all = vec![format!("std::is_convertible<{read}, {}>", self.view)];
                if self.bytes {
                    all.push(format!("::SCE::Forge::Detail::Lasting<{read}>"));
                }
                all
            }
        }
    }

    /// Whether the encode's read of it as a member of a group's arm, `read`,
    /// and what it does with what the read returns throws nothing: keeping a
    /// body or a list and reading it as sources of its codec, or converting
    /// anything else to its type.
    fn member_nothrow(&self, read: &str) -> String {
        let kept = format!(
            "noexcept(std::declval<::SCE::Forge::Detail::Kept<decltype({read})>&>().keep({read}))"
        );
        // A chain an arm holds is kept as the list it holds, its entries read
        // as sources of its entry codec.
        // An optional one, where it is there.
        if let Some(chain) = &self.chain {
            let list = chain.list_read();
            let inner = if chain.optional {
                format!("({read})->{list}")
            } else {
                format!("{read}.{list}")
            };
            return format!(
                "noexcept(std::declval<::SCE::Forge::Detail::Kept<::SCE::Forge::Detail::InnerRead<decltype({read})>>&>().keep({inner})) && {}::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename ::SCE::Forge::Detail::ChainOf<std::decay_t<decltype({read})>>::type::list_type>>::value",
                chain.namespace
            );
        }
        // Two lists of one count: each list's entries are read as sources of
        // its entry codec.
        if let Some(lists) = &self.aligned {
            let held = format!("const std::decay_t<decltype({read})>&");
            return format!(
                "{kept} && {}::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype(std::declval<{held}>().first())>>>::value && {}::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype(std::declval<{held}>().second())>>>::value",
                lists.first_namespace, lists.second_namespace
            );
        }
        match (&self.body, &self.list) {
            (Some(held), _) => format!(
                "{kept} && {}::is_nothrow_source<std::decay_t<decltype({read})>>::value",
                held.namespace
            ),
            (None, Some((held, _))) => format!(
                "{kept} && {}::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype({read})>>>::value",
                held.namespace
            ),
            (None, None) => format!(
                "noexcept(::SCE::Forge::Detail::converts_to<{}>({read}))",
                self.view
            ),
        }
    }
}

/// The C++ text of the codec `plan`, whose class is `struct_name`; the plan is
/// one [`unlowered`] answers `None` for.
pub(super) fn render(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> CppSource {
    let elements: Vec<Presented<'_>> = plan
        .elements
        .iter()
        .map(|e| {
            let law = super::rust_source::law_type(&plan.roles, struct_name, &e.field.id);
            Presented::of(e, l, imports, law)
        })
        .collect();
    // Each group of elements one gate decides, held as one element.
    let groups: Vec<GroupPart<'_>> = plan
        .groups()
        .iter()
        .map(|group| GroupPart::of(plan, group, l, imports, struct_name))
        .collect();
    // The elements and the groups in the order their fields are declared.
    let mut parts: Vec<(usize, Part)> = elements
        .iter()
        .enumerate()
        .map(|(index, e)| {
            let at = plan
                .model
                .fields
                .iter()
                .position(|f| f.id == e.field.id)
                .expect("an element is a field of the codec");
            (at, Part::Element(index))
        })
        .chain(
            groups
                .iter()
                .enumerate()
                .map(|(index, g)| (g.at, Part::Group(index))),
        )
        .collect();
    parts.sort_by_key(|(at, _)| *at);
    let parts: Vec<Part> = parts.into_iter().map(|(_, part)| part).collect();
    let part_name = |part: &Part| match part {
        Part::Element(index) => elements[*index].name.as_str(),
        Part::Group(index) => groups[*index].name.as_str(),
    };
    let mut before = String::new();
    before.push_str(&format!(
        "/// The elements a value of `{struct_name}` presents, each read by name: the\n\
         /// member function where the type has one, otherwise a function the type's own\n\
         /// namespace declares (docs/adr/0081, step 4).\n\
         namespace source {{\n"
    ));
    let variant = VariantPart::of(plan, imports, struct_name);
    // What a source presents, each read by name: every element and group, and
    // the body a variant holds the arm in.
    let mut readers: Vec<&str> = parts.iter().map(part_name).collect();
    if variant.is_some() {
        readers.push("body");
    }
    // What a source presents for a group presents each of its members by name,
    // and an arm that holds two lists of one count presents the two by one.
    let mut reader_names: Vec<&str> = readers.clone();
    for g in &groups {
        let pairs = g.aligned().map(|(_, m, _)| m);
        for m in g.members.iter().chain(pairs) {
            if !reader_names.contains(&m.name.as_str()) {
                reader_names.push(m.name.as_str());
            }
        }
    }
    for name in &reader_names {
        before.push_str(&format!(
            "namespace detail_{name} {{\n\
             void {name}() = delete;\n\
             template <class S>\n\
             constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.{name}())) -> decltype(s.{name}()) {{\n\
             \x20   return s.{name}();\n\
             }}\n\
             template <class S>\n\
             constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept({name}(s))) -> decltype({name}(s)) {{\n\
             \x20   return {name}(s);\n\
             }}\n\
             struct Read {{\n\
             \x20   template <class S>\n\
             \x20   constexpr auto operator()(const S& s) const\n\
             \x20       noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{{}})))\n\
             \x20       -> decltype(read(s, ::SCE::Forge::PreferMember{{}})) {{\n\
             \x20       return read(s, ::SCE::Forge::PreferMember{{}});\n\
             \x20   }}\n\
             }};\n\
             }}  // namespace detail_{name}\n\
             /// The `{name}` element of a source.\n\
             inline constexpr detail_{name}::Read {name}{{}};\n"
        ));
    }
    before.push_str("}  // namespace source\n\n");

    let names: Vec<&str> = readers.clone();
    // The kind of entry each gate on a chain's entries asks for, which the
    // chain each arm holds is a `Chain` of (docs/adr/0065 §6 R6b).
    for g in &groups {
        let Some(chain) = g.group.chain() else {
            continue;
        };
        let ns = g
            .members
            .iter()
            .find(|m| m.field.id == chain)
            .and_then(|m| m.list.as_ref())
            .map(|(held, _)| held.namespace.clone())
            .expect("the chain whose memberships are gates is a list of the group");
        for (_, key) in g.group.members() {
            // A bare entry is its identifier alone, which the encode writes
            // where the value holds its place (R6a).
            let (doc, named) = if g.group.member_is_bare() {
                (
                    "which is all of its value (docs/adr/0065 §6 R6a)",
                    named_entry(plan, imports, chain, key),
                )
            } else {
                (
                    "which a value holds where it stands (docs/adr/0065 §6 R6b)",
                    String::new(),
                )
            };
            before.push_str(&format!(
                "/// The kind of entry `{chain}.has(0x{key:X})` names: an entry whose identifier\n\
                 /// is 0x{key:X}, {doc}.\n\
                 struct {key_type} {{\n\
                 \x20   template <class E>\n\
                 \x20   [[nodiscard]] static bool matches(const E& entry) {{\n\
                 \x20       return {identifier} == 0x{key:X}u;\n\
                 \x20   }}\n\
                 {named}\
                 }};\n\n",
                key_type = super::rust_source::key_type(struct_name, chain, key),
                identifier = entry_identifier(plan, l, &ns, chain, "entry")
            ));
        }
    }
    for g in &groups {
        before.push_str(&g.class_text());
    }
    before.push_str(&law_definitions(plan, imports, &elements, l, struct_name));
    for trait_text in elements
        .iter()
        .filter_map(|e| e.list_trait().or_else(|| e.body_trait()))
    {
        before.push_str(&trait_text);
    }
    for g in &groups {
        before.push_str(&g.traits());
    }
    if let Some(variant) = &variant {
        before.push_str(&variant.traits());
    }
    before.push_str(&format!(
        "/// Whether `S` presents every element of `{struct_name}`, each convertible to\n\
         /// its type (a C++17 trait, ADR 0059 §3).\n\
         template <class S, class = void>\n\
         struct is_source : std::false_type {{}};\n"
    ));
    if names.is_empty() {
        before.push_str("template <class S>\nstruct is_source<S, void> : std::true_type {};\n\n");
    } else {
        let reads: Vec<String> = names
            .iter()
            .map(|n| format!("decltype(source::{n}(std::declval<const S&>()))"))
            .collect();
        let mut converts: Vec<String> = parts
            .iter()
            .map(|part| match part {
                Part::Element(index) => elements[*index].source_requirement(),
                Part::Group(index) => groups[*index].source_requirement(),
            })
            .collect();
        if variant.is_some() {
            converts.push(
                "is_body<std::decay_t<decltype(source::body(std::declval<const S&>()))>>"
                    .to_string(),
            );
        }
        before.push_str(&format!(
            "template <class S>\n\
             struct is_source<S, std::void_t<{}>>\n\
             \x20   : std::conjunction<{}> {{}};\n\n",
            reads.join(", "),
            converts.join(", ")
        ));
    }

    // The encode throws only what a source's reads throw, with the conversion
    // of each to the element's type: the sink's writes throw nothing, and the
    // generated value's accessors are `noexcept`, so its own `encode` is. A
    // type that is no source is not asked, so the encode's `static_assert` is
    // what names its missing elements.
    let nothrow = if names.is_empty() {
        "true".to_string()
    } else {
        parts
            .iter()
            .map(|part| match part {
                Part::Element(index) => elements[*index].nothrow_requirement(),
                Part::Group(index) => groups[*index].nothrow_requirement(),
            })
            .chain(variant.as_ref().map(VariantPart::nothrow_requirement))
            .collect::<Vec<_>>()
            .join(" && ")
    };
    before.push_str(&format!(
        "/// Whether reading every element of a source `S` throws nothing.\n\
         template <class S, class = void>\n\
         struct is_nothrow_source : std::false_type {{}};\n\
         template <class S>\n\
         struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>\n\
         \x20   : std::bool_constant<{nothrow}> {{}};\n\n"
    ));
    // The flag inputs the caller hands the encode (docs/adr/0065 §6 R4a), one
    // byte each, named as the template's `encode` names them.
    let input_name = |name: &str| filters::to_snake_case(name.to_string());
    let mut input_params: String = plan
        .given
        .iter()
        .map(|input| format!(", std::uint8_t {}", input_name(&input.name)))
        .collect();
    // The flags a parent may set in place of the ones a value presents, which
    // a chain holding this codec sets on each entry it writes; a caller that
    // sets none passes nothing.
    if !plan.overridable.is_empty() {
        let mut members = String::new();
        for (carrier, def) in &plan.overridable {
            members.push_str(&format!(
                "    /// The `{}` flag of `{}`, where a parent sets it.\n    std::optional<bool> {};\n",
                def.name,
                carrier.id,
                override_member(l, &carrier.id, &def.name)
            ));
        }
        before.push_str(&format!(
            "/// One-bit flags of `{struct_name}` a parent may set in place of the ones a value\n\
             /// presents, such as the flag a chain whose entries are `{struct_name}` ends them\n\
             /// on, which the chain derives from where each entry stands.\n\
             struct Overrides {{\n{members}}};\n\n"
        ));
        input_params.push_str(", const Overrides& overrides = Overrides{}");
    }
    before.push_str(&format!(
        "/// The encode of `{struct_name}` over any source of it: each element read\n\
         /// through `source`, written as the plan lays it out (docs/adr/0081). It\n\
         /// refuses nothing; what it returns is the sink's own failure, and it\n\
         /// throws only what a read of `src` throws.\n\
         struct Wire {{\n\
         \x20   template <class S>\n\
         \x20   [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w{input_params}) noexcept(is_nothrow_source<S>::value) {{\n\
         \x20       static_assert(is_source<S>::value, \"S presents every element of {struct_name}: {}\");\n",
        if names.is_empty() { "none".to_string() } else { names.join(", ") }
    ));
    if names.is_empty() {
        before.push_str("        (void)src;\n        (void)w;\n");
    }
    // The body a variant holds the arm in is read once, as the source of the
    // arm's codec it presents.
    if variant.is_some() {
        before.push_str("        const auto& _r_body = source::body(src);\n");
    }
    // An input is read where a gate or a held codec needs it, which some
    // layouts never do.
    for input in &plan.given {
        before.push_str(&format!("        (void){};\n", input_name(&input.name)));
    }
    // A byte string is read once and viewed within its bound, and an optional
    // value is read once and tested before it is written: what the read
    // returns lives to the end of the encode, so a view never outlives it. A
    // body is read once, as the source of its codec it is, and kept as read; a
    // list likewise, as the bounded list of entry sources it is.
    for e in &elements {
        if e.body.is_some() || e.list.is_some() {
            before.push_str(&format!(
                "        const auto& _r_{name} = source::{name}(src);\n",
                name = e.name
            ));
        } else if e.kept() {
            before.push_str(&format!(
                "        const auto& _r_{name} = source::{name}(src);\n\
                 \x20       const {presented} _e_{name} = _r_{name};\n",
                name = e.name,
                presented = e.presented()
            ));
        }
    }
    // A group is read once, and each of its members bound to an optional local
    // from the arm the value holds where that arm holds it, so each statement
    // that writes a member is the one that writes an optional element.
    for g in &groups {
        before.push_str(&g.bind_members());
    }
    // An element by its field: one the value presents alone, or a member of a
    // group it presents.
    let element = |f: &CodecField| {
        elements
            .iter()
            .chain(groups.iter().flat_map(|g| g.members.iter()))
            .find(|e| e.field.id == f.id)
    };
    let optional = |f: &CodecField| element(f).is_some_and(|e| e.optional);
    // Each input the value says (docs/adr/0065 §6 R4a) that a gate of the
    // codec reads, read off what it holds under the name the gate reads it by,
    // as [`said_input`] reads it for a parent; one no gate reads is left to the
    // parent that asks for it.
    for said in plan.said.iter().filter(|said| said.read) {
        let name = input_name(&said.input.name);
        before.push_str(&format!(
            "        const std::uint8_t {name} = {};\n        (void){name};\n",
            said_input(
                plan,
                imports,
                &elements,
                &groups,
                variant.as_ref(),
                &said.input.name,
                Read::Encode
            )
        ));
    }

    // What the encode derives (docs/adr/0081 §3): a number held whole in a
    // field, as the local `_d_<field>`, and the flags of a carrier, folded into
    // the carrier as `_c_<carrier>`, each bit-range replaced. A number is the
    // count of the bytes it measures as the wire states it, which the bytes'
    // bound keeps within what holds it; where a gate leaves the bytes out, a
    // field gated beside them holds zero and a flag keeps the bits the value
    // presents. A presence flag is set exactly when the field it gates is
    // there, or exactly when it is not for a field gated on the flag's clear
    // state; fields one group holds are there together, so the first says it.
    let mut folds: std::collections::BTreeMap<&str, Vec<Fold>> = std::collections::BTreeMap::new();
    let mut encoded: Vec<(&ElementId, &Number<'_>)> = Vec::new();
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            (ElementId::Flag { field, flag }, Derive::Presence(presence)) => {
                let (gated, when_clear) =
                    presence.of.first().expect("a presence flag gates a field");
                let local = element(gated)
                    .expect("a presence flag gates an element the value presents")
                    .local();
                let test = if *when_clear {
                    format!("!{local}.has_value()")
                } else {
                    format!("{local}.has_value()")
                };
                folds.entry(field.as_str()).or_default().push(Fold {
                    flag: flag.clone(),
                    bits: format!("static_cast<std::uint64_t>({test})"),
                    present: None,
                });
            }
            (element_id, Derive::Number(number)) => {
                let [measure] = number.measures.as_slice() else {
                    unreachable!("`unlowered` refuses a number of more than one measure");
                };
                let (size, present) = match &measure.of {
                    Measured::Bytes {
                        field: bytes,
                        offset,
                    } => {
                        let name = l.codec_field_id(&bytes.id);
                        let arith = match offset {
                            Offset::None => String::new(),
                            Offset::More { by } => format!(" + {by}u"),
                            Offset::Less { by, .. } => format!(" - {by}u"),
                        };
                        if measure.gated {
                            (
                                format!("_e_{name}->size(){arith}"),
                                Some(format!("_e_{name}.has_value()")),
                            )
                        } else {
                            (format!("_e_{name}.size(){arith}"), None)
                        }
                    }
                    // The input a body always there says, read off the source
                    // the value presents it by (docs/adr/0065 §6 R4a).
                    // Where a gate leaves the body out, it says nothing: a
                    // field beside it holds zero and a flag keeps its bits.
                    Measured::Says { body, input, .. } => {
                        let e = element(body)
                            .filter(|e| e.body.is_some())
                            .expect("a body that says an input is a body the value presents");
                        let held = e.body.as_ref().expect("filtered to a body");
                        let says = |source: &str| {
                            format!("{}::input_{}({source})", held.namespace, input_name(input))
                        };
                        let local = e.local();
                        if e.optional {
                            (
                                format!(
                                    "({local}.has_value() ? {} : std::uint8_t{{0}})",
                                    says(&format!("*{local}"))
                                ),
                                Some(format!("{local}.has_value()")),
                            )
                        } else {
                            (says(&local), None)
                        }
                    }
                    // How many entries the list holds, as the source presents
                    // it, within its bound; where a gate leaves the list out, a
                    // field beside it holds zero and a flag keeps its bits.
                    Measured::Entries { list } => {
                        let counted = element(list)
                            .filter(|e| e.list.is_some())
                            .expect("a count of entries counts a list the value presents");
                        if counted.optional {
                            (
                                format!("{}.size()", counted.entries()),
                                Some(format!("{}.has_value()", counted.local())),
                            )
                        } else {
                            (format!("{}.size()", counted.entries()), None)
                        }
                    }
                    // The input the arm the value holds says, read through
                    // the arm's codec off the source the body presents it as
                    // (the catch-all's, off its body); where the value holds
                    // an arm that does not say it, a field beside it holds
                    // zero and a flag keeps the bits the value presents
                    // (docs/adr/0065 §6 R4a).
                    Measured::ArmSays { arms } => {
                        let variant = variant
                            .as_ref()
                            .expect("an input an arm says is said where the codec has a variant");
                        let local = match element_id {
                            ElementId::Field(id) => format!("_s_{}", l.codec_field_id(id)),
                            ElementId::Flag { field, flag } => format!(
                                "_s_{}_{}",
                                l.codec_field_id(field),
                                filters::to_snake_case(flag.clone())
                            ),
                            ElementId::Variant => {
                                unreachable!("an input an arm says is bound to a field or a flag")
                            }
                        };
                        let there = measure.gated.then(|| format!("{local}_there"));
                        let mut cases = String::new();
                        for said in arms {
                            let (held, tag) = &variant.arms[said.arm];
                            let index = said.arm;
                            let arm = match tag {
                                Some(_) => format!("*std::get_if<{index}>(&_r_body)"),
                                None => format!("std::get_if<{index}>(&_r_body)->body"),
                            };
                            let mark = there
                                .as_ref()
                                .map(|t| format!(" {t} = true;"))
                                .unwrap_or_default();
                            cases.push_str(&format!(
                                "            case {index}: {local} = static_cast<std::size_t>({}::input_{}({arm}));{mark} break;\n",
                                held.namespace,
                                input_name(&said.input)
                            ));
                        }
                        before.push_str(&format!("        std::size_t {local} = 0;\n"));
                        if let Some(there) = &there {
                            before.push_str(&format!("        bool {there} = false;\n"));
                        }
                        before.push_str(&format!(
                            "        switch (_r_body.index()) {{\n\
                             {cases}\
                             \x20           default: break;\n\
                             \x20       }}\n"
                        ));
                        (local, there)
                    }
                    // Counted below, once the carriers its body may be handed
                    // are as the encode writes them.
                    Measured::Encoded { .. } => {
                        encoded.push((element_id, number));
                        continue;
                    }
                    // The start of the list the value holds, which its law
                    // holds every entry to (docs/adr/0065 §6 R8); where a gate
                    // leaves the list out, a field beside it holds zero and a
                    // flag keeps the bits the value presents.
                    Measured::Start { list, input } => {
                        let held = element(list)
                            .filter(|e| e.law.is_some())
                            .expect("a list's start is the start of a list held to a law");
                        let laws = super::rust_source::list_laws(&plan.roles, &list.id);
                        let start = format!("{}.start()", held.entries());
                        let start = if laws.len() == 1 {
                            start
                        } else {
                            let at = laws
                                .iter()
                                .position(|law| law.binder.input == *input)
                                .expect("the list holds the input by its law");
                            format!("std::get<{at}>({start})")
                        };
                        (
                            format!("static_cast<std::size_t>({start})"),
                            held.optional
                                .then(|| format!("{}.has_value()", held.local())),
                        )
                    }
                };
                match (element_id, &number.into) {
                    (ElementId::Field(_), NumberInto::Field { field, absent, .. }) => {
                        let ty = l.type_name(&field.sce_type);
                        // Where the content is not, zero, or what the group
                        // that holds the field holds on that side, bound from
                        // the arm the value holds (zero where it holds none, as
                        // the Rust encode reads it).
                        let otherwise = match absent {
                            Absent::Group => {
                                format!("_e_{}.value_or({ty}{{0}})", l.codec_field_id(&field.id))
                            }
                            _ => format!("{ty}{{0}}"),
                        };
                        let value = match &present {
                            None => format!("static_cast<{ty}>({size})"),
                            Some(there) => {
                                format!("{there} ? static_cast<{ty}>({size}) : {otherwise}")
                            }
                        };
                        before.push_str(&format!(
                            "        const {ty} _d_{} = {value};\n",
                            l.codec_field_id(&field.id)
                        ));
                    }
                    (ElementId::Flag { field, flag }, NumberInto::Flag) => {
                        folds.entry(field.as_str()).or_default().push(Fold {
                            flag: flag.clone(),
                            bits: format!("static_cast<std::uint64_t>({size})"),
                            present,
                        });
                    }
                    _ => unreachable!("`unlowered` refuses a number into what holds none"),
                }
            }
            // The tag of the arm the value holds: the value the document gives
            // a named arm, and the tag the catch-all holds.
            (element_id, Derive::VariantTag) => {
                let variant = variant
                    .as_ref()
                    .expect("a variant's tag is derived only where the codec has one");
                let mut cases = String::new();
                for (index, (_, tag)) in variant.arms.iter().enumerate() {
                    cases.push_str(&match tag {
                        Some(tag) => format!(
                            "            case {index}: _t_body = {tag}ull; break;\n"
                        ),
                        None => format!(
                            "            case {index}: _t_body = static_cast<std::uint64_t>(std::get_if<{index}>(&_r_body)->tag); break;\n"
                        ),
                    });
                }
                before.push_str(&format!(
                    "        std::uint64_t _t_body = 0;\n\
                     \x20       switch (_r_body.index()) {{\n\
                     {cases}\
                     \x20           default: break;\n\
                     \x20       }}\n"
                ));
                match element_id {
                    // The catch-all's tag is held in a whole byte, so it is
                    // masked to the flag's bits, which leave the carrier's other
                    // bits as they are.
                    ElementId::Flag { field, flag } => {
                        let width = plan
                            .model
                            .fields
                            .iter()
                            .find(|f| f.id == *field)
                            .and_then(|carrier| carrier.flags.iter().find(|d| d.name == *flag))
                            .map(|def| def.width)
                            .expect("a tag is derived into a flag of a carrier of the codec");
                        let mask = if width >= 64 {
                            u64::MAX
                        } else {
                            (1u64 << width) - 1
                        };
                        folds.entry(field.as_str()).or_default().push(Fold {
                            flag: flag.clone(),
                            bits: format!("(_t_body & 0x{mask:X}ull)"),
                            present: None,
                        });
                    }
                    ElementId::Field(id) => {
                        let field = plan
                            .model
                            .fields
                            .iter()
                            .find(|f| f.id == *id)
                            .expect("a tag is derived into a field of the codec");
                        let ty = l.type_name(&field.sce_type);
                        before.push_str(&format!(
                            "        const {ty} _d_{} = static_cast<{ty}>(_t_body);\n",
                            l.codec_field_id(&field.id)
                        ));
                    }
                    ElementId::Variant => {
                        unreachable!("`unlowered` refuses a variant whose caller hands its tag")
                    }
                }
            }
            // The tag of the arm each embedded body's variant holds, masked to
            // the flag's bits, the catch-all's the tag it holds; nothing where
            // a gate leaves the body out. The arms are in the order the body's
            // variant holds them, the catch-all last.
            (ElementId::Flag { field, flag }, Derive::ParentTag(tag)) => {
                let local = format!(
                    "_p_{}_{}",
                    l.codec_field_id(field),
                    filters::to_snake_case(flag.clone())
                );
                before.push_str(&format!("        std::uint64_t {local} = 0;\n"));
                for term in &tag.terms {
                    let e = element(term.body)
                        .expect("a parent tag is filled by a body the value presents");
                    let held = e
                        .body
                        .as_ref()
                        .expect("a parent tag is filled by an embedded body");
                    let mut cases = String::new();
                    for (index, arm) in term.arms.iter().enumerate() {
                        cases.push_str(&format!(
                            "                case {index}: {local} |= 0x{:X}ull; break;\n",
                            arm.tag
                        ));
                    }
                    if term.catch_all {
                        let index = term.arms.len();
                        cases.push_str(&format!(
                            "                case {index}: {local} |= static_cast<std::uint64_t>(std::get_if<{index}>(&_v)->tag) & 0x{:X}ull; break;\n",
                            term.mask
                        ));
                    }
                    let (open, read) = if e.optional {
                        (
                            format!("if ({}.has_value()) ", e.local()),
                            format!("*{}", e.local()),
                        )
                    } else {
                        (String::new(), e.local())
                    };
                    before.push_str(&format!(
                        "        {open}{{\n\
                         \x20           const auto& _v = {}::source::body({read});\n\
                         \x20           switch (_v.index()) {{\n\
                         {cases}\
                         \x20               default: break;\n\
                         \x20           }}\n\
                         \x20       }}\n",
                        held.namespace
                    ));
                }
                folds.entry(field.as_str()).or_default().push(Fold {
                    flag: flag.clone(),
                    bits: local,
                    present: None,
                });
            }
            // Set by the walk of the chain that holds the entry, not here.
            (ElementId::Flag { .. }, Derive::ChainContinuation) => {}
            // A flag of a disjunction that decides a group: the value the arm
            // the value holds gives it, or the one it has while the value
            // holds no arm (docs/adr/0065 §6 R4c).
            (ElementId::Flag { field, flag }, Derive::ArmFlag { group, gate }) => {
                let g = &groups[*group];
                let value = g.gate_value(&g.group.gates[*gate], &format!("_r_{}", g.name));
                folds.entry(field.as_str()).or_default().push(Fold {
                    flag: flag.clone(),
                    bits: format!("static_cast<std::uint64_t>({value})"),
                    present: None,
                });
            }
            _ => unreachable!("`unlowered` refuses a derivation the printer does not lower"),
        }
    }
    // A flag a parent sets in place of the one the value presents (the flag a
    // chain's entries end on), folded last: what the parent says is what the
    // wire holds.
    for (carrier, def) in &plan.overridable {
        let name = override_member(l, &carrier.id, &def.name);
        folds.entry(carrier.id.as_str()).or_default().push(Fold {
            flag: def.name.clone(),
            bits: format!("static_cast<std::uint64_t>(*overrides.{name})"),
            present: Some(format!("overrides.{name}.has_value()")),
        });
    }
    for (carrier_id, flags) in &folds {
        let carrier = plan
            .model
            .fields
            .iter()
            .find(|f| f.id == *carrier_id)
            .expect("a derived flag is a flag of a carrier of the codec");
        let ty = l.type_name(&carrier.sce_type);
        let name = l.codec_field_id(&carrier.id);
        // The carrier as the source presents it, each derived bit-range
        // replaced in turn: one statement a bit-range, under the test that its
        // bytes are there where a gate may leave them out.
        let bits = format!("_f_{name}");
        before.push_str(&format!(
            "        std::uint64_t {bits} = static_cast<std::uint64_t>(source::{name}(src));\n"
        ));
        for fold in flags {
            let def = carrier
                .flags
                .iter()
                .find(|d| d.name == fold.flag)
                .expect("a derived flag is a flag of its carrier");
            let width_mask = if def.width >= 64 {
                u64::MAX
            } else {
                (1u64 << def.width) - 1
            };
            let mask = width_mask << def.bit;
            let placed = if def.bit == 0 {
                fold.bits.clone()
            } else {
                format!("({} << {})", fold.bits, def.bit)
            };
            let set = format!("{bits} = ({bits} & ~0x{mask:X}ull) | {placed};");
            before.push_str(&match &fold.present {
                None => format!("        {set}\n"),
                Some(there) => format!("        if ({there}) {{\n            {set}\n        }}\n"),
            });
        }
        before.push_str(&format!(
            "        const {ty} _c_{name} = static_cast<{ty}>({bits});\n"
        ));
    }

    // The value a field is written as: the number the encode derived, the
    // carrier with its derived flags, the present value of an optional element
    // (read only where it is tested present), or what the source presents; a
    // value of an enum, as the enum's carrier.
    let read = |f: &CodecField| {
        let name = l.codec_field_id(&f.id);
        let value = if derived_number(plan, &f.id) {
            format!("_d_{name}")
        } else if folds.contains_key(f.id.as_str()) {
            format!("_c_{name}")
        } else if optional(f) {
            format!("(*_e_{name})")
        } else {
            format!("source::{name}(src)")
        };
        l.codec_carrier_expr(&f.sce_type, &value)
    };
    // A text's UTF-8 bytes are `char`s, written as the bytes they are.
    let bytes_write = |f: &CodecField| {
        let name = l.codec_field_id(&f.id);
        let at = if optional(f) { "->" } else { "." };
        let data = if element(f).is_some_and(|e| e.text) {
            format!("reinterpret_cast<const std::uint8_t*>(_e_{name}{at}data())")
        } else {
            format!("_e_{name}{at}data()")
        };
        format!(
            "        if (auto _e = w.write_bytes({data}, _e_{name}{at}size()); _e) return _e;\n"
        )
    };
    // A gate, tested on the carriers as the encode writes them and on the
    // inputs it is handed.
    let gate_test = |f: &CodecField| {
        let gate = f
            .present_if
            .as_ref()
            .expect("a field written where its gate holds is gated");
        let bit = |read: String, mask: u64, negate: bool| {
            format!(
                "((static_cast<std::uint64_t>({read}) & 0x{mask:X}ull) {} 0)",
                if negate { "==" } else { "!=" }
            )
        };
        plan.clauses(gate)
            .iter()
            .map(|clause| match clause {
                Clause::Flag {
                    carrier,
                    bit: at,
                    negate,
                } => bit(read(carrier), 1u64 << at, *negate),
                Clause::Input { name, negate } => bit(input_name(name), 1, *negate),
                // A membership a group's gate is: what the arm the value holds
                // says of it (docs/adr/0065 §6 R6b).
                Clause::Member {
                    held: Some((group, Some(gate))),
                    negate,
                    ..
                } => {
                    let g = &groups[*group];
                    let value = g.gate_value(&g.group.gates[*gate], &format!("_r_{}", g.name));
                    format!("({value} {} 0)", if *negate { "==" } else { "!=" })
                }
                Clause::Member { .. } => {
                    unreachable!("`unlowered` refuses a membership no group's gate is")
                }
            })
            .collect::<Vec<_>>()
            .join(" || ")
    };
    // What a held codec is handed for each input it takes (docs/adr/0065 §6
    // R4a), in the order it declares them: bits of a carrier as the encode
    // writes it, or an input of this codec's own, passed on.
    let handed = |alias: &str| -> String {
        plan.handed
            .get(alias)
            .map(|inputs| {
                inputs
                    .iter()
                    .map(|handed| match &handed.from {
                        Some(HandedFrom::Carrier {
                            carrier,
                            bit,
                            width,
                        }) => {
                            let mask = if *width >= 64 {
                                u64::MAX
                            } else {
                                (1u64 << width) - 1
                            };
                            format!(
                                ", static_cast<std::uint8_t>((static_cast<std::uint64_t>({}) >> {bit}) & 0x{mask:X}ull)",
                                read(carrier)
                            )
                        }
                        Some(HandedFrom::Input { name }) => format!(", {}", input_name(name)),
                        None => unreachable!("`unlowered` refuses a body handed an input nothing binds"),
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    // How many bytes a body encodes to, counted by its encode over a sink that
    // keeps none; its encode refuses nothing, so the count is all it returns.
    for (element_id, number) in encoded {
        let [measure] = number.measures.as_slice() else {
            unreachable!("`unlowered` refuses a number of more than one measure");
        };
        let Measured::Encoded { body, .. } = &measure.of else {
            unreachable!("only an encoded length is counted here");
        };
        let measured = element(body).expect("an encoded length measures a body the value presents");
        let held = measured
            .body
            .as_ref()
            .expect("an encoded length measures a body the value presents");
        let name = l.codec_field_id(&body.id);
        // A body a gate leaves out encodes to nothing, so the count stays zero.
        let count = format!(
            "(void){}::Wire::encode_source({}, _k_{name}{});",
            held.namespace,
            if measured.optional {
                format!("*_r_{name}")
            } else {
                format!("_r_{name}")
            },
            handed(&held.alias)
        );
        before.push_str(&format!("        ::SCE::Forge::CountingSink _k_{name};\n"));
        before.push_str(&if measured.optional {
            format!("        if (_r_{name}) {{\n            {count}\n        }}\n")
        } else {
            format!("        {count}\n")
        });
        match (element_id, &number.into) {
            (ElementId::Field(_), NumberInto::Field { field, .. }) => {
                let ty = l.type_name(&field.sce_type);
                before.push_str(&format!(
                    "        const {ty} _d_{} = static_cast<{ty}>(_k_{name}.position());\n",
                    l.codec_field_id(&field.id)
                ));
            }
            _ => unreachable!("`unlowered` refuses an encoded length into what holds none"),
        }
    }
    // The statements of a guarded write, inside the test that guards it.
    let guarded = |test: String, body: String| {
        let mut out = format!("        if ({test}) {{\n");
        for line in body.lines() {
            out.push_str(&format!("    {line}\n"));
        }
        out.push_str("        }\n");
        out
    };
    for write in &plan.program {
        match write {
            Write::Packed(fields) => {
                let parts: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        let bits = f.fixed_bits().unwrap_or(8);
                        let mask = (1u64 << bits) - 1;
                        let at = f.bit_offset.unwrap_or(0);
                        format!(
                            "((static_cast<std::uint32_t>({}) & 0x{mask:X}u) << {at})",
                            read(f)
                        )
                    })
                    .collect();
                before.push_str(&format!(
                    "        if (auto _e = w.write_u8(static_cast<std::uint8_t>({})); _e) return _e;\n",
                    parts.join(" | ")
                ));
            }
            Write::Fixed(f) => before.push_str(&fixed_write(plan, f, &read(f))),
            Write::Tail { field, pad } => {
                // A field the layout aligns follows zeros up to its offset.
                if *pad {
                    before.push_str(&format!(
                        "        while (w.position() < {}) {{\n\
                         \x20           if (auto _e = w.write_u8(0); _e) return _e;\n\
                         \x20       }}\n",
                        field.byte_offset
                    ));
                }
                before.push_str(&bytes_write(field));
            }
            // A field in turn; a list among them is walked as its bit size
            // says, `unlowered` having refused every walk it does not lower.
            Write::Streamed { field, guard, list } => {
                let body = match &field.bit_size {
                    // A body, written by its codec's encode over the source the
                    // value read it as, with the inputs that codec is handed.
                    // One a gate decides is written where it is present, as the
                    // source it holds then.
                    BitSize::Embed => {
                        let embedded = element(field)
                            .expect("an embedded field is a body the value presents");
                        let held = embedded
                            .body
                            .as_ref()
                            .expect("an embedded field is a body the value presents");
                        let name = l.codec_field_id(&field.id);
                        let read = if embedded.optional {
                            format!("*_r_{name}")
                        } else {
                            format!("_r_{name}")
                        };
                        format!(
                            "        if (auto _e = {}::Wire::encode_source({read}, w{}); _e) return _e;\n",
                            held.namespace,
                            handed(&held.alias)
                        )
                    }
                    // A list, each entry written in turn by its codec's encode
                    // over the source the list holds it as; one a gate decides,
                    // over the list it holds where it is present.
                    BitSize::Repeat { .. } | BitSize::TlvChain { .. } => {
                        let walked = element(field)
                            .filter(|e| e.list.is_some())
                            .expect("a list field is a list the value presents");
                        let (held, _) = walked
                            .list
                            .as_ref()
                            .expect("a list field is a list the value presents");
                        let name = l.codec_field_id(&field.id);
                        let entries = walked.entries();
                        let ends = list.as_ref().and_then(|list| list.ends.as_ref());
                        // A bare entry's chain is walked as slots: the others in
                        // order, the named entry where the value holds its place
                        // (docs/adr/0065 §6 R6a).
                        let slots = list.as_ref().and_then(|list| list.slots);
                        if let Some(key) = slots {
                            let key = super::rust_source::key_type(struct_name, &field.id, key);
                            let (overrides, arg) = match ends {
                                Some((carrier, flag)) => (
                                    format!(
                                        "\x20               {ns}::Overrides _o_{name}{{}};\n\
                                         \x20               _o_{name}.{member} = _i + 1 < _k_{name};\n",
                                        ns = held.namespace,
                                        member = override_member(l, carrier, flag)
                                    ),
                                    format!(", _o_{name}"),
                                ),
                                None => (String::new(), String::new()),
                            };
                            format!(
                                "        {{\n\
                                 \x20           const std::size_t _k_{name} = {entries}.size() + (_p_{name} ? 1u : 0u);\n\
                                 \x20           auto _n_{name} = {entries}.begin();\n\
                                 \x20           for (std::size_t _i = 0; _i < _k_{name}; ++_i) {{\n\
                                 {overrides}\
                                 \x20               if (_p_{name} && *_p_{name} == _i) {{\n\
                                 \x20                   if (auto _e = {ns}::Wire::encode_source({key}::named(), w{handed}{arg}); _e) return _e;\n\
                                 \x20               }} else {{\n\
                                 \x20                   if (auto _e = {ns}::Wire::encode_source(*_n_{name}, w{handed}{arg}); _e) return _e;\n\
                                 \x20                   ++_n_{name};\n\
                                 \x20               }}\n\
                                 \x20           }}\n\
                                 \x20       }}\n",
                                ns = held.namespace,
                                handed = handed(&held.alias)
                            )
                        } else {
                        match ends {
                            // A chain sets on each entry but the last the flag
                            // its entries end on, in place of the one the entry
                            // presents.
                            Some((carrier, flag)) => format!(
                                "        {{\n\
                                 \x20           std::size_t _left_{name} = {entries}.size();\n\
                                 \x20           for (const auto& _x : {entries}) {{\n\
                                 \x20               --_left_{name};\n\
                                 \x20               {ns}::Overrides _o_{name}{{}};\n\
                                 \x20               _o_{name}.{member} = _left_{name} != 0;\n\
                                 \x20               if (auto _e = {ns}::Wire::encode_source(_x, w{handed}, _o_{name}); _e) return _e;\n\
                                 \x20           }}\n\
                                 \x20       }}\n",
                                ns = held.namespace,
                                member = override_member(l, carrier, flag),
                                handed = handed(&held.alias)
                            ),
                            None => format!(
                                "        for (const auto& _x : {entries}) {{\n\
                                 \x20           if (auto _e = {}::Wire::encode_source(_x, w{}); _e) return _e;\n\
                                 \x20       }}\n",
                                held.namespace,
                                handed(&held.alias)
                            ),
                        }
                        }
                    }
                    BitSize::Fixed { .. } => fixed_write(plan, field, &read(field)),
                    BitSize::Tail | BitSize::LengthRef => bytes_write(field),
                    BitSize::Vle { width_bits } => format!(
                        "        if (auto _e = w.write_vle_u{width_bits}(static_cast<std::uint{width_bits}_t>({})); _e) return _e;\n",
                        read(field)
                    ),
                };
                before.push_str(&match guard {
                    Guard::Always => body,
                    // Where the value holds it.
                    Guard::Present => guarded(
                        element(field)
                            .expect("a field written where the value holds it is an element")
                            .local(),
                        body,
                    ),
                    // Derived whole and gated: where its gate holds, as the
                    // carriers the encode writes say.
                    Guard::Gate => guarded(gate_test(field), body),
                });
            }
            // The arm the value holds, written by its codec's encode over the
            // source the body presents it as; the catch-all, over its body.
            Write::Variant => {
                let variant = variant
                    .as_ref()
                    .expect("a variant's arm is written only where the codec has one");
                let mut cases = String::new();
                for (index, (held, tag)) in variant.arms.iter().enumerate() {
                    let arm = match tag {
                        Some(_) => format!("*std::get_if<{index}>(&_r_body)"),
                        None => format!("std::get_if<{index}>(&_r_body)->body"),
                    };
                    cases.push_str(&format!(
                        "            case {index}:\n\
                         \x20               if (auto _e = {}::Wire::encode_source({arm}, w{}); _e) return _e;\n\
                         \x20               break;\n",
                        held.namespace,
                        handed(&held.alias)
                    ));
                }
                before.push_str(&format!(
                    "        switch (_r_body.index()) {{\n\
                     {cases}\
                     \x20           default: break;\n\
                     \x20       }}\n"
                ));
            }
        }
    }
    before.push_str("        return std::nullopt;\n    }\n};\n\n");
    // Each input the value says, read off any source of it: what a parent that
    // holds this codec derives the flag it binds to the input from
    // (docs/adr/0065 §6 R4a).
    for said in &plan.said {
        let name = input_name(&said.input.name);
        // A value that can say nothing of the input answers no value then
        // (docs/adr/0065 §6 R4e).
        let (answer, nothing) = if plan.roles.says_input_partly(&said.input.name) {
            (
                "std::optional<std::uint8_t>",
                ", and nothing where the value says nothing of it",
            )
        } else {
            ("std::uint8_t", "")
        };
        // An input the arms say is read off the body, read once.
        let body = if matches!(
            plan.roles.inputs[&said.input.name],
            InputSource::EveryArm { .. } | InputSource::SomeArms { .. }
        ) {
            "    const auto& _r_body = source::body(src);\n"
        } else {
            ""
        };
        before.push_str(&format!(
            "/// The flag input `{}` a source of `{struct_name}` says, which its encode is not\n\
             /// handed (docs/adr/0065 §6 R4a){nothing}.\n\
             template <class S>\n\
             [[nodiscard]] {answer} input_{name}(const S& src) noexcept(is_nothrow_source<S>::value) {{\n\
             \x20   static_assert(is_source<S>::value, \"S presents every element of {struct_name}\");\n\
             {body}\
             \x20   return {};\n\
             }}\n\n",
            said.input.name,
            said_input(
                plan,
                imports,
                &elements,
                &groups,
                variant.as_ref(),
                &said.input.name,
                Read::Source
            )
        ));
    }

    // A value that holds a byte string copies it in, which may throw and is no
    // constant expression in C++17; a value of integers alone is both.
    // A body is held by value and moved in; the codec it holds may hold bytes,
    // so a value holding a body is made as one holding bytes is.
    // A variant's arm is a codec's value, which may hold bytes, so a value
    // holding one is made as one holding bytes is.
    // A group that holds a byte string or a text is made as a value holding
    // bytes is.
    let holds_bytes = variant.is_some()
        || elements
            .iter()
            .any(|e| e.bytes || e.body.is_some() || e.list.is_some())
        || groups.iter().any(GroupPart::holds_bytes);
    let mut members = String::new();
    members.push_str(&format!("    {struct_name}() = default;\n"));
    if !names.is_empty() {
        let mut params: Vec<String> = parts
            .iter()
            .map(|part| match part {
                Part::Element(index) => {
                    let e = &elements[*index];
                    format!("{} {}", e.presented(), e.name)
                }
                Part::Group(index) => {
                    let g = &groups[*index];
                    format!("{} {}", g.holds(), g.name)
                }
            })
            .collect();
        let mut inits: Vec<String> = parts
            .iter()
            .map(|part| match part {
                Part::Element(index) => {
                    let e = &elements[*index];
                    if e.body.is_some() {
                        format!("{}(std::move({}))", member(&e.name), e.name)
                    } else {
                        format!("{}({})", member(&e.name), e.name)
                    }
                }
                // A group that holds bytes is moved in, as a body is.
                Part::Group(index) => {
                    let g = &groups[*index];
                    if g.holds_bytes() {
                        format!("{}(std::move({}))", member(&g.name), g.name)
                    } else {
                        format!("{}({})", member(&g.name), g.name)
                    }
                }
            })
            .collect();
        // The arm the value holds comes last, after the fields before it.
        if let Some(variant) = &variant {
            params.push(format!("{} body", variant.type_name));
            inits.push(format!("{}(std::move(body))", member("body")));
        }
        let (constexpr, noexcept) = if holds_bytes {
            ("", "")
        } else {
            ("constexpr ", " noexcept")
        };
        members.push_str(&format!(
            "    /// The value of each element it presents, in declaration order (ADR 0059 §5).\n\
             \x20   {constexpr}{struct_name}({}){noexcept}\n\
             \x20       : {} {{}}\n",
            params.join(", "),
            inits.join(", ")
        ));
    }
    for part in &parts {
        let e = match part {
            Part::Element(index) => &elements[*index],
            // A group is presented by reference, as the value holds it.
            Part::Group(index) => {
                let g = &groups[*index];
                let which = if g.optional() {
                    "where its gate leaves them in"
                } else {
                    "as the arm the value holds"
                };
                let constexpr = if g.holds_bytes() { "" } else { "constexpr " };
                members.push_str(&format!(
                    "    /// The fields `{label}` decides, {which}.\n\
                     \x20   [[nodiscard]] {constexpr}const {holds}& {name}() const noexcept {{ return {held}; }}\n",
                    label = g.label(),
                    holds = g.holds(),
                    name = g.name,
                    held = member(&g.name)
                ));
                continue;
            }
        };
        let name = &e.name;
        let held = member(name);
        // A body is presented by reference: the value holds it whole, as an
        // optional value where a gate decides it.
        if e.body.is_some() {
            let what = if e.optional {
                "the body it holds where a gate leaves it in"
            } else {
                "the body it holds"
            };
            members.push_str(&format!(
                "    /// The `{name}` element, {what}.\n\
                 \x20   [[nodiscard]] const {}& {name}() const noexcept {{ return {held}; }}\n",
                e.holds()
            ));
            continue;
        }
        // A list is viewed within its bound, as the value holds it; one a gate
        // decides, where it is present.
        if e.list.is_some() {
            let (what, body) = if e.optional {
                (
                    "the entries it holds where a gate leaves them in",
                    format!(
                        "if (!{held}) {{ return std::nullopt; }} return {}(*{held});",
                        e.view
                    ),
                )
            } else {
                ("the entries it holds", format!("return {held};"))
            };
            members.push_str(&format!(
                "    /// The `{name}` element, {what}.\n\
                 \x20   [[nodiscard]] {} {name}() const noexcept {{ {body} }}\n",
                e.presented()
            ));
            continue;
        }
        let constexpr = if e.bytes { "" } else { "constexpr " };
        // An optional byte string is viewed where it is present.
        let body = if e.bytes && e.optional {
            format!(
                "if (!{held}) {{ return std::nullopt; }} return {}(*{held});",
                e.view
            )
        } else {
            format!("return {held};")
        };
        let what = if e.optional {
            format!("The `{name}` element, where a gate leaves it in.")
        } else {
            format!("The `{name}` element.")
        };
        members.push_str(&format!(
            "    /// {what}\n\
             \x20   [[nodiscard]] {constexpr}{} {name}() const noexcept {{ {body} }}\n",
            e.presented()
        ));
    }
    if let Some(variant) = &variant {
        members.push_str(&format!(
            "    /// The arm the value holds, each arm's body the value of its codec.\n\
             \x20   [[nodiscard]] const {}& body() const noexcept {{ return {}; }}\n",
            variant.type_name,
            member("body")
        ));
    }

    // A default-made value holds what its declared flag values give its
    // carrier (`<sce:flag value=..>`), which is what lets a variant's marked
    // default arm encode its own wire tag (RFC variant-default-uniformity);
    // a byte string holds the fewest bytes its bound admits, a value of an enum
    // the first value its document declares, and every other element holds its
    // zero.
    let mut private = String::new();
    if !names.is_empty() {
        private.push_str("\nprivate:\n");
        for part in &parts {
            let e = match part {
                Part::Element(index) => &elements[*index],
                // A group a value made by default holds is none, or the arm
                // present while no gate holds.
                Part::Group(index) => {
                    let g = &groups[*index];
                    private.push_str(&format!(
                        "    {} {}{};\n",
                        g.holds(),
                        member(&g.name),
                        g.default_init()
                    ));
                    continue;
                }
            };
            let init = match &e.field.sce_type {
                // A closed set does not hold its carrier's zero unless it
                // declares it, so a value of an enum starts at the first value
                // its document declares, which every enum has.
                SceType::Enum(r) if !e.optional => format!("{{{}}}", l.enum_default_expr(&r.alias)),
                _ if !e.optional && e.field.flags.iter().any(|fl| fl.value.is_some()) => {
                    cpp_flag_default_init(e.field)
                }
                _ => "{}".to_string(),
            };
            private.push_str(&format!("    {} {}{init};\n", e.holds(), member(&e.name)));
        }
        // A value made by default holds the arm marked `default="true"`, which
        // encodes its own tag; else the first.
        if let Some(variant) = &variant {
            let init = match variant.default_index {
                Some(index) => format!("{{std::in_place_index_t<{index}>{{}}}}"),
                None => "{}".to_string(),
            };
            private.push_str(&format!(
                "    {} {}{init};\n",
                variant.type_name,
                member("body")
            ));
        }
    }

    // The decode makes the value by its constructor from the locals it read:
    // a byte string through its bound's one door (where it is present, for one
    // a gate decides), and a derived number read and dropped, since the value
    // derives it. A carrier keeps the flags it was read with.
    let mut decoded = String::new();
    // A list the decode read is held to its bound by the same door, past which
    // it has too many entries (where it is present, for one a gate decides).
    // A list held to a law is then held to it from the start the frame gave,
    // past which an entry says what its place does not ask of it.
    for e in elements.iter().filter(|e| e.law.is_some()) {
        let (name, view) = (&e.name, &e.view);
        let start = decoded_law_start(plan, l, &e.field.id, view);
        let make = |indent: &str, list: &str, into: &str| {
            format!(
                "{indent}const auto _l_{name} = {view}::list_type::make({list});\n\
                 {indent}if (!_l_{name}) {{\n\
                 {indent}    cursor.refuse(::SCE::Forge::CodecError::TooManyElements);\n\
                 {indent}    return std::nullopt;\n\
                 {indent}}}\n\
                 {indent}{into} = {view}::make({start}, *_l_{name});\n\
                 {indent}if (!_b_{name}) {{\n\
                 {indent}    cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);\n\
                 {indent}    return std::nullopt;\n\
                 {indent}}}\n"
            )
        };
        if e.optional {
            decoded.push_str(&format!(
                "        std::optional<{view}> _b_{name};\n\
                 \x20       if ({name}) {{\n\
                 {}\
                 \x20       }}\n",
                make("            ", &format!("*{name}"), &format!("_b_{name}"))
            ));
        } else {
            decoded.push_str(&make("        ", name, &format!("const auto _b_{name}")));
        }
    }
    // A list a group holds likewise, where its gate leaves it in, before the
    // group is made of it.
    let lists = elements.iter().map(|e| (e, e.optional)).chain(
        groups
            .iter()
            .flat_map(|g| g.members.iter())
            .map(|m| (m, m.read_optionally())),
    );
    for (e, optional) in lists.filter(|(e, _)| e.list.is_some() && e.law.is_none()) {
        let (name, view) = (&e.name, &e.view);
        if optional {
            decoded.push_str(&format!(
                "        std::optional<{view}> _b_{name};\n\
                 \x20       if ({name}) {{\n\
                 \x20           _b_{name} = {view}::make(*{name});\n\
                 \x20           if (!_b_{name}) {{\n\
                 \x20               cursor.refuse(::SCE::Forge::CodecError::TooManyElements);\n\
                 \x20               return std::nullopt;\n\
                 \x20           }}\n\
                 \x20       }}\n"
            ));
        } else {
            decoded.push_str(&format!(
                "        const auto _b_{name} = {view}::make({name});\n\
                 \x20       if (!_b_{name}) {{\n\
                 \x20           cursor.refuse(::SCE::Forge::CodecError::TooManyElements);\n\
                 \x20           return std::nullopt;\n\
                 \x20       }}\n"
            ));
        }
    }
    // A byte string or a text a group holds is held to its bound likewise,
    // where its gate leaves it in, before the group is made of it.
    let bytes = elements.iter().map(|e| (e, e.optional)).chain(
        groups
            .iter()
            .flat_map(|g| g.members.iter())
            .map(|m| (m, m.read_optionally())),
    );
    for (e, optional) in bytes.filter(|(e, _)| e.bytes) {
        let (name, view) = (&e.name, &e.view);
        if optional {
            decoded.push_str(&format!(
                "        std::optional<{view}> _b_{name};\n\
                 \x20       if ({name}) {{\n\
                 \x20           _b_{name} = {view}::make(*{name});\n\
                 \x20           if (!_b_{name}) {{\n\
                 \x20               cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);\n\
                 \x20               return std::nullopt;\n\
                 \x20           }}\n\
                 \x20       }}\n"
            ));
        } else {
            decoded.push_str(&format!(
                "        const auto _b_{name} = {view}::make({name});\n\
                 \x20       if (!_b_{name}) {{\n\
                 \x20           cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);\n\
                 \x20           return std::nullopt;\n\
                 \x20       }}\n"
            ));
        }
    }
    for field in &plan.model.fields {
        if derived_number(plan, &field.id) {
            decoded.push_str(&format!("        (void){};\n", l.codec_field_id(&field.id)));
        }
    }
    // A group is made of the members the decode read: those one gate leaves
    // in or out together, or those of the arm the gates hold.
    for g in &groups {
        decoded.push_str(&g.decoded(plan.model, l));
    }
    decoded.push_str(&format!("        {struct_name} _decoded{{\n"));
    for part in &parts {
        let e = match part {
            Part::Element(index) => &elements[*index],
            Part::Group(index) => {
                let g = &groups[*index];
                let value = if g.holds_bytes() {
                    format!("std::move(_g_{})", g.name)
                } else {
                    format!("_g_{}", g.name)
                };
                decoded.push_str(&format!("            /*{name}=*/{value},\n", name = g.name));
                continue;
            }
        };
        let name = &e.name;
        let value = match (e.bytes, e.optional) {
            (true, false) => format!("*_b_{name}"),
            (true, true) => format!("_b_{name}"),
            // A body the decode read is moved into the value.
            (false, _) if e.body.is_some() => format!("std::move({name})"),
            // A list is made from the view of the entries the decode read,
            // where a gate leaves them in for one it decides.
            (false, true) if e.list.is_some() => format!("_b_{name}"),
            (false, false) if e.list.is_some() => format!("*_b_{name}"),
            (false, _) => name.clone(),
        };
        decoded.push_str(&format!("            /*{name}=*/{value},\n"));
    }
    // The arm the decode read is moved in, last.
    if variant.is_some() {
        decoded.push_str("            /*body=*/std::move(body),\n");
    }
    decoded.push_str("        };\n");

    // Each text ends at its last line; the template places it on a line of
    // its own.
    let line = |s: String| s.trim_end_matches('\n').to_string();
    CppSource {
        before: line(before),
        members: line(members),
        private: line(private),
        decoded: line(decoded),
    }
}

/// The bounds of a list, a byte string or a text as the arguments of its
/// `Bounded`: the most it holds, `::SCE::Forge::kUnbounded` where the protocol
/// states none (docs/adr/0077 §7), then the fewest where that is not none.
fn bounds_args(bound: &Bound) -> String {
    let most = bound.most.map_or_else(
        || "::SCE::Forge::kUnbounded".to_string(),
        |most| most.to_string(),
    );
    if bound.least == 0 {
        most
    } else {
        format!("{most}, {}", bound.least)
    }
}

/// The write of the fixed field `f`, whose value is `value`: a byte or less at
/// its bit offset, or one of the sink's whole widths in the field's byte order.
fn fixed_write(plan: &EncodePlan<'_>, f: &CodecField, value: &str) -> String {
    let bits = f.fixed_bits().unwrap_or(8);
    let endian = f.endian.unwrap_or(plan.model.default_endian);
    let call = match bits {
        b if b < 8 => {
            let at = f.bit_offset.unwrap_or(0);
            format!(
                "w.write_u8(static_cast<std::uint8_t>((static_cast<std::uint32_t>({value}) & 0x{:X}u) << {at}))",
                (1u64 << b) - 1
            )
        }
        8 => format!("w.write_u8(static_cast<std::uint8_t>({value}))"),
        b => {
            let side = if matches!(endian, Endian::Little) {
                "le"
            } else {
                "be"
            };
            format!("w.write_u{b}_{side}(static_cast<std::uint{b}_t>({value}))")
        }
    };
    format!("        if (auto _e = {call}; _e) return _e;\n")
}
