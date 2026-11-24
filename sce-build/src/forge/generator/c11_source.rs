// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C11 printer of a codec's encode plan (docs/adr/0081 §2, step 4).
//!
//! A C11 value presents its elements to its encode through a table of
//! functions, `{codec}_source_t`, each handed the context pointer the table is
//! passed with: the interface of C, as an operations table is, which a type of
//! any layout joins by a table of its own, and which a build places in
//! read-only memory once for each type, so a source costs an MCU no RAM. The
//! encode is a function over any source, `{codec}_encode_source(src, ctx, w)`:
//! it reads each element once, through its function, and derives every other
//! element of the frame from them as the plan says, never from the value
//! (docs/adr/0058 §1). A value of the codec's struct is one source, by the
//! table `{codec}_value_source()` answers, whose context is the value, and its
//! `encode` is the encode over that source.
//!
//! The printer reads the model only through the plan and lowers its forms one
//! landing at a time (§6): [`unlowered`] names the first form a codec's plan
//! uses that it does not print, and such a codec keeps the template's encode
//! whole, so no codec has two encodes.

use super::rust_source::{gates_label, group_accessor, list_laws};
use super::*;
use crate::forge::codec_roles::GateVar;
use crate::forge::codec_roles::{ElementId, InputSource, InputValue};
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_failure::EncodeFailure;
use crate::forge::encode_plan::{
    Absent, Against, Automaton, Bound, CheckSite, Clause, CodecPlan, Derive, Element, EncodePlan,
    EntryId, Guard, HandedFrom, InputCheck, Law, Measure, Measured, Narrow, Number, NumberInto,
    Offset, ParentTag, Presence, Value, Write,
};
use std::collections::{BTreeMap, BTreeSet};

mod group;
pub(super) mod source_case;

/// The member the codec's struct holds its variant's arm in, and the function
/// of the table of a source that presents the arm (`{snake}_body_ref_t`).
const BODY: &str = crate::scxml_identifier::VARIANT_MEMBER;

/// The constant of `{codec}_body_kind_t` that names the arm whose codec is
/// imported as `arm`, or the catch-all where `arm` is `None`: the kind the
/// struct of the codec named `codec` holds its variant's arm by, and the kind
/// a source presents the arm by. A known arm and the catch-all are apart even
/// where one codec is both, as the struct holds them apart. The arm is named
/// by [`variant_arm_name`], as every backend names it, in upper snake case.
pub(super) fn arm_kind(codec: &str, arm: Option<&str>) -> String {
    format!(
        "{}_BODY_KIND_{}",
        to_upper_snake(codec),
        to_upper_snake(&variant_arm_name(arm))
    )
}

/// The member of the variant's union that holds the body of the arm whose
/// codec is imported as `arm`, or of the catch-all where `arm` is `None`: the
/// arm's [`variant_arm_name`] in snake case, the catch-all's with `_body`
/// after it, so that no alias names it.
pub(super) fn arm_member(arm: Option<&str>) -> String {
    let name = filters::to_snake_case(variant_arm_name(arm));
    match arm {
        Some(_) => name,
        None => format!("{name}_body"),
    }
}

/// The type of the kinds of the arms of the variant of the codec `snake`.
pub(super) fn kind_type(snake: &str) -> String {
    format!("{snake}_body_kind_t")
}

/// `text`, whole lines, inside the guard of the build feature macro `feature`
/// where it has one (docs/adr/0072), so a build without the feature holds
/// nothing of it.
fn feature_guarded(feature: Option<&str>, text: &str) -> String {
    match feature {
        Some(feature) => format!("#if {feature}\n{text}#endif\n"),
        None => text.to_string(),
    }
}

/// The arms of a codec's variant as the C11 printer names them, in the plan's
/// order, the catch-all last. The codec's struct holds the arm in
/// `{snake}_variant_t`: its `kind`, the tag the catch-all holds and the arm's
/// body in a member of a union. A source presents it as `{snake}_body_ref_t`,
/// the same `kind` and tag beside the source of the arm's body, in a member of
/// the same name.
struct Arms {
    arms: Vec<ArmName>,
}

/// One arm of [`Arms`].
struct ArmName {
    /// The prefix of the symbols of its codec ([`body_prefix`]).
    prefix: String,
    /// The import alias of its codec.
    alias: String,
    /// Its constant of `{snake}_body_kind_t` ([`arm_kind`]).
    kind: String,
    /// The member of the union that holds its body ([`arm_member`]).
    member: String,
    /// The tag it is chosen by, or `None` for the catch-all, which holds the
    /// tag it was read with.
    tag: Option<u64>,
    /// The macro of the build feature it is compiled under (docs/adr/0072).
    feature: Option<String>,
}

impl Arms {
    /// The arms of the variant of `plan`.
    fn of(plan: &EncodePlan<'_>) -> Self {
        Arms {
            arms: plan
                .arms
                .iter()
                .map(|arm| {
                    let named = arm.tag.map(|_| arm.codec);
                    ArmName {
                        prefix: body_prefix(arm.codec),
                        alias: arm.codec.to_string(),
                        kind: arm_kind(&plan.model.name, named),
                        member: arm_member(named),
                        tag: arm.tag,
                        feature: arm.feature.map(c_feature_macro),
                    }
                })
                .collect(),
        }
    }

    /// `text`, whole lines, as a build holds arm `i`: inside the guard of its
    /// feature where it has one.
    fn guarded(&self, i: usize, text: &str) -> String {
        feature_guarded(self.arms[i].feature.as_deref(), text)
    }

    /// The `case` of each arm of `arms`, by their places, at the indentation
    /// of a `switch` inside an encode's own block, each running `body` of the
    /// arm and then `break`, and a `default` that does nothing: `kind` is the
    /// kind switched on.
    fn switch(&self, kind: &str, arms: &[usize], body: impl Fn(&ArmName) -> String) -> String {
        let mut cases = String::new();
        for &i in arms {
            let arm = &self.arms[i];
            cases.push_str(&self.guarded(
                i,
                &format!(
                    "        case {}:\n{}            break;\n",
                    arm.kind,
                    body(arm)
                ),
            ));
        }
        format!("    switch ({kind}) {{\n{cases}        default:\n            break;\n    }}\n")
    }

    /// The places of every arm.
    fn all(&self) -> Vec<usize> {
        (0..self.arms.len()).collect()
    }
}

/// The source of the body of `arm` as `held`, a `{snake}_body_ref_t`, presents
/// it: its table and its context.
fn arm_source(held: &str, arm: &ArmName) -> (String, String) {
    (
        format!("{held}.arm.{}.table", arm.member),
        format!("{held}.arm.{}.ctx", arm.member),
    )
}

/// Whether the codec imported as `alias` says its flag input `input` in some
/// of its values only (docs/adr/0065 §6 R4e), so that its function of it
/// answers an optional value.
fn says_partly(imports: &[ImportContext], alias: &str, input: &str) -> bool {
    imports.iter().any(|import| {
        import.alias == alias && import.codec_partly_says_inputs.iter().any(|s| s == input)
    })
}

/// The first form of `plan` the C11 printer does not print, as the clause
/// that names it, or `None` where it prints the codec whole.
pub(super) fn unlowered(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Option<String> {
    unlowered_forms(plan, imports).into_iter().next()
}

/// Every form of `plan` the C11 printer does not print, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole.
///
/// It prints a positional codec whose elements are integers, flag sets, enums,
/// byte strings, texts, and bodies and lists of codecs it prints, a list held
/// to the law its entries say the inputs it hands them by ([`law_text`]),
/// always there, decided by a gate of a carrier's flags, or a number the value
/// states while the content it measures is absent, a group a gate of a flag or
/// an input decides, of numbers, enums, byte strings, texts, a number the arm
/// states where its content is absent, and bodies and lists of codecs it
/// prints held to no law, held as the struct of its arm ([`group`]), and a
/// variant of arms of codecs it prints, whatever holds its tag, an arm under a
/// build feature inside the guard of its macro; whose every derived element
/// is a number measured off byte strings, texts, lists and bodies or read off
/// what a body, the arm the value holds or a list's start says, in a field or
/// in a flag of a carrier, a flag set where the elements it gates are there, a
/// flag a group's arm sets, the tag of the arm the value holds, or the tag of
/// the arm a body holds. A flag input the value does not say is a parameter of
/// the encode, and one it says, off a field gated on it alone, a group, a
/// body, the arm the value holds or a list's start, is read off the source; a
/// body, an arm and a list's entries are handed each input their import binds,
/// and a chain's entries each input nothing binds as whether the entry before
/// was of the kind it waits for. Its encode
/// refuses, before its first byte, a number past what holds it, measures of
/// one number that disagree, a number
/// the value states beside content that says another, a gate it cannot derive
/// that disagrees with the element it gates, an input a held codec says that
/// disagrees with what it is handed, and what a body or an entry it holds
/// refuses. Its writes are the fixed bytes and the byte strings of a
/// positional layout and, of a streamed one, whole fixed fields,
/// variable-length integers, byte strings, bodies, and lists and chains,
/// walked entry by entry, a chain closed by an entry flag setting it on each
/// entry but the last through the entry's overrides, then the arm the value
/// holds; and the one-bit flags a parent may set in place of the value's are
/// overrides it folds in last. A CBOR map and a
/// content-line component keep their own templates' encodes. `imports` are the
/// codec's own, which say whether a codec it holds is printed
/// ([`ImportContext::c11_printed`]).
pub(super) fn unlowered_forms(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    match plan {
        CodecPlan::Positional(plan) => positional_forms(plan, imports),
        CodecPlan::Cbor(_) => vec!["a CBOR map".to_string()],
        CodecPlan::ContentLine(_) => vec!["a content-line component".to_string()],
    }
}

/// The C11 prefix of the symbols of the codec a body of `plan` holds as
/// `alias`, where the C11 backend prints it from its plan: its header then
/// declares `{prefix}_encode_source`, `{prefix}_value_source` and
/// `{prefix}_source_ref_t`. The prefix is the one the template names the
/// body's encode by (`resolve_variant_arm_encoder`).
fn printed_body(imports: &[ImportContext], alias: &str) -> Option<String> {
    imports
        .iter()
        .any(|import| import.alias == alias && import.c11_printed)
        .then(|| body_prefix(alias))
}

/// The prefix [`printed_body`] answers, of the codec imported as `alias`.
fn body_prefix(alias: &str) -> String {
    filters::to_snake_case(alias.to_string())
}

/// The names of the parameters an encode over a source takes besides its flag
/// inputs, which no input may take.
const ENCODE_PARAMETERS: [&str; 4] = ["src", "ctx", "w", "overrides"];

/// The C name of the flag input `name`: a parameter of the encode and of the
/// decode, and the local an input the value says is read into.
fn input_name(name: &str) -> String {
    filters::to_snake_case(name.to_string())
}

/// Whether the codec imported as `alias` is handed an input its import binds
/// nothing to, which only a chain supplying it from the entry before hands.
fn unbound_handed(plan: &EncodePlan<'_>, alias: &str) -> bool {
    plan.handed
        .get(alias)
        .is_some_and(|inputs| inputs.iter().any(|handed| handed.from.is_none()))
}

/// Whether the walk of the chain `field`, of entries of the codec imported as
/// `alias`, hands each entry every input its import binds nothing to: each is
/// one the chain supplies from the entry before (`ListWrite::previous`).
fn walk_supplies(plan: &EncodePlan<'_>, field: &CodecField, alias: &str) -> bool {
    let Some(walk) = plan.program.iter().find_map(|write| match write {
        Write::Streamed {
            field: f,
            list: Some(walk),
            ..
        } if f.id == field.id => Some(walk),
        _ => None,
    }) else {
        return false;
    };
    plan.handed.get(alias).is_some_and(|inputs| {
        inputs
            .iter()
            .filter(|handed| handed.from.is_none())
            .all(|handed| {
                walk.previous
                    .iter()
                    .any(|(input, _)| *input == handed.input)
            })
    })
}

/// What of the gate of `field` the encode does not test: it tests a bit of a
/// carrier as the encode writes it and a flag input ([`gate_test`]).
fn gate_form(plan: &EncodePlan<'_>, field: &CodecField) -> Option<String> {
    let gate = field.present_if.as_ref()?;
    plan.clauses(gate)
        .iter()
        .any(|clause| matches!(clause, Clause::Member { .. }))
        .then(|| format!("field '{}': a gate of a chain's membership", field.id))
}

/// Whether the encode reads the flag input `name` the value says, so that it
/// reads it off the elements first: a gate or a check of the plan reads it
/// (`Said::read`), a gate the encode holds a field to tests it, or a codec the
/// value holds is handed it.
fn encode_reads(plan: &EncodePlan<'_>, name: &str) -> bool {
    let tested = |field: &CodecField| {
        field.present_if.as_ref().is_some_and(|gate| {
            plan.clauses(gate)
                .iter()
                .any(|clause| matches!(clause, Clause::Input { name: n, .. } if n == name))
        })
    };
    plan.said
        .iter()
        .any(|said| said.input.name == name && said.read)
        || plan.gate_checks.iter().copied().any(tested)
        || plan
            .handed
            .values()
            .flatten()
            .any(|handed| matches!(&handed.from, Some(HandedFrom::Input { name: n }) if n == name))
}

/// [`unlowered_forms`] of a positional codec.
fn positional_forms(plan: &EncodePlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    let m = plan.model;
    let mut forms = Vec::new();
    // The arm the value holds is presented as `{snake}_body_ref_t`, its kind
    // beside the source of its body, and written by the arm's codec's own
    // encode over that source, which a codec the C11 backend prints has,
    // handed each input its import binds ([`Arms`]). An arm a build feature
    // decides is inside the guard of its macro wherever it is named.
    if m.variant.is_some() {
        if plan.elements.iter().any(|e| e.field.id == BODY) {
            forms.push(format!(
                "field '{BODY}': an element named as the variant's arm is presented"
            ));
        }
        for arm in &plan.arms {
            if printed_body(imports, arm.codec).is_none() {
                forms.push(format!(
                    "arm `{}`: a codec the C11 backend does not print",
                    arm.codec
                ));
            }
            if unbound_handed(plan, arm.codec) {
                forms.push(format!(
                    "arm `{}`: a codec handed an input its import binds nothing to",
                    arm.codec
                ));
            }
        }
    }
    // A flag input the encode is handed is a parameter of it, named as the
    // decode names it, after the sink and before the overrides; one the value
    // says is read off the source ([`said_input`]), as the input's own
    // function `{codec}_input_<name>` reads it for a parent.
    for input in &m.flag_inputs {
        let name = input_name(&input.name);
        if ENCODE_PARAMETERS.contains(&name.as_str()) || name.starts_with('_') {
            forms.push(format!(
                "flag input '{}': a name the encode's own parameters or locals take",
                input.name
            ));
        }
    }
    for said in &plan.said {
        match plan.roles.inputs.get(&said.input.name) {
            // Said by the field gated on it, the body it hands it to, the arm
            // of the group it decides the value holds, or the arms that say it.
            Some(
                InputSource::Gate { .. }
                | InputSource::Forward { .. }
                | InputSource::Group
                | InputSource::SomeArms { .. },
            ) => {}
            // Said by the arm the value holds through its codec's own function
            // of it ([`arm_inputs`]); every arm says a number, which an arm
            // that says it in some of its values only does not.
            Some(InputSource::EveryArm { arms }) => {
                for (alias, input) in arms {
                    if says_partly(imports, alias, input) {
                        forms.push(format!(
                            "flag input '{}': an input every arm says, `{alias}` in some of its \
                             values only",
                            said.input.name
                        ));
                    }
                }
            }
            // Said by the start of the list it is handed on to, which the
            // list its maker made holds (docs/adr/0065 §6 R8): a list the
            // value presents alone, which says nothing only where a gate
            // leaves it out.
            Some(InputSource::List { list, .. }) => {
                let gated = element_of(plan, list).map(|e| e.presence == Presence::Gated);
                let partly = plan.roles.says_input_partly(&said.input.name);
                if gated.is_none() || (gated == Some(true) && !partly) {
                    forms.push(format!(
                        "flag input '{}': an input the start of a list the value does not \
                         present alone says",
                        said.input.name
                    ));
                }
            }
            None => forms.push(format!(
                "flag input '{}': an input nothing the value holds says",
                said.input.name
            )),
        }
        // A value that says it in some of its values only answers no number
        // the encode could test or hand on.
        if plan.roles.says_input_partly(&said.input.name) && encode_reads(plan, &said.input.name) {
            forms.push(format!(
                "flag input '{}': an input the value says in part that the encode reads",
                said.input.name
            ));
        }
    }
    for element in &plan.elements {
        let id = &element.field.id;
        match &element.value {
            Value::Scalar(t) if t.int_bit_width().is_some() => {}
            Value::Scalar(SceType::Enum(_)) => {}
            Value::Scalar(t) => forms.push(format!("field '{id}': a `{}` field", t.as_attr())),
            // Held in the struct's array beside the length member the C11
            // struct names `{id}_len`, which no element of its own may name.
            Value::Bytes(_) | Value::Text(_) => {
                let member = format!("{id}_len");
                if plan.elements.iter().any(|other| other.field.id == member) {
                    forms.push(format!(
                        "field '{id}': a byte string whose length member another element names"
                    ));
                }
            }
            // Presented as the table and the context of its codec's source,
            // and written by that codec's own encode over it, which a codec
            // the C11 backend prints has, handed each input its import binds
            // ([`handed_args`]).
            Value::Body { codec } => {
                if printed_body(imports, codec).is_none() {
                    forms.push(format!(
                        "field '{id}': a body of `{codec}`, which the C11 backend does not print"
                    ));
                }
                if unbound_handed(plan, codec) {
                    forms.push(format!(
                        "field '{id}': a codec handed an input its import binds nothing to"
                    ));
                }
            }
            // Presented as a walk of its entries, each a source of its codec,
            // and written entry by entry by that codec's own encode, which a
            // codec the C11 backend prints has. The value holds the entries in
            // an array beside the length member `{id}_len`, which its walk
            // `{codec}_value_{id}_begin` and `_next` read, names no element
            // of its own may take.
            // A list held to a law is made by a maker that holds each entry to
            // it, the start of each input it gives every entry alike a member
            // of the list and of the value beside it ([`law_text`],
            // [`start_member`]). A chain hands an entry an input nothing binds
            // from the entry before it, as its walk says.
            Value::List { codec, law, .. } => {
                if printed_body(imports, codec).is_none() {
                    forms.push(format!(
                        "field '{id}': a list of `{codec}`, which the C11 backend does not print"
                    ));
                }
                if unbound_handed(plan, codec) && !walk_supplies(plan, element.field, codec) {
                    forms.push(format!(
                        "field '{id}': a codec handed an input its import binds nothing to"
                    ));
                }
                let mut taken = vec![
                    format!("{id}_len"),
                    format!("{id}_begin"),
                    format!("{id}_next"),
                ];
                taken.extend(
                    law.inputs
                        .iter()
                        .map(|i| start_member(id, &input_name(&i.input))),
                );
                if plan
                    .elements
                    .iter()
                    .any(|other| taken.contains(&other.field.id))
                {
                    forms.push(format!(
                        "field '{id}': a list whose length member or walk another element names"
                    ));
                }
            }
        }
        // An element a gate decides, and a number the value states while its
        // content is absent, are presented as the optional value of what they
        // hold.
    }
    // A group of fields a gate of a flag or an input decides is held as a
    // tagged struct of its arms ([`group`]): a number or an enum as its type,
    // a byte string or a text in the buffer of its bound, a body of a codec
    // this printer prints as its struct and a list of one as the runtime's
    // list of its entries, each read by the decode in place, where its own
    // gate holds, into the arm the gates it read select.
    for group in plan.groups() {
        let label = gates_label(group);
        if group
            .gates
            .iter()
            .any(|g| matches!(g, GateVar::Member { .. }))
        {
            forms.push(format!("group `{label}`: a group of a chain's memberships"));
        }
        if group.aligned.is_some() {
            forms.push(format!("group `{label}`: two lists of one count"));
        }
        // The decode tests each flag on the carrier it read into the value.
        for gate in &group.gates {
            if let GateVar::Flag { carrier, .. } = gate {
                if !element_of(plan, carrier).is_some_and(|e| e.presence == Presence::Always) {
                    forms.push(format!(
                        "group `{label}`: a gate of a carrier the value does not always present"
                    ));
                }
            }
        }
        let name = group_accessor(group);
        if m.fields
            .iter()
            .any(|f| filters::to_snake_case(f.id.clone()) == name)
        {
            forms.push(format!(
                "group `{label}`: a group named as a field of the codec is"
            ));
        }
        for id in group.fields() {
            let Some(field) = m.fields.iter().find(|f| f.id == id) else {
                forms.push(format!(
                    "group `{label}`: a member `{id}` the codec does not declare"
                ));
                continue;
            };
            match plan.value(field) {
                Value::Scalar(t) if t.int_bit_width().is_some() => {}
                Value::Scalar(SceType::Enum(_)) => {}
                Value::Scalar(t) => forms.push(format!(
                    "group `{label}`: a member `{id}`, a `{}` field",
                    t.as_attr()
                )),
                Value::Bytes(_) | Value::Text(_) => {}
                Value::Body { codec } => {
                    if printed_body(imports, codec).is_none() {
                        forms.push(format!(
                            "group `{label}`: a member `{id}`, a body of `{codec}`, which the C11 \
                             backend does not print"
                        ));
                    }
                    if unbound_handed(plan, codec) {
                        forms.push(format!(
                            "group `{label}`: a member `{id}`, a codec handed an input its import \
                             binds nothing to"
                        ));
                    }
                }
                Value::List { codec, law, .. } => {
                    if printed_body(imports, codec).is_none() {
                        forms.push(format!(
                            "group `{label}`: a member `{id}`, a list of `{codec}`, which the C11 \
                             backend does not print"
                        ));
                    }
                    if unbound_handed(plan, codec) {
                        forms.push(format!(
                            "group `{label}`: a member `{id}`, a codec handed an input its import \
                             binds nothing to"
                        ));
                    }
                    if !law.is_none() {
                        forms.push(format!(
                            "group `{label}`: a member `{id}`, a list held to a law"
                        ));
                    }
                    if field.is_tlv_chain() {
                        forms.push(format!("group `{label}`: a member `{id}`, a chain"));
                    }
                }
            }
            if field.quantity.is_some() {
                forms.push(format!(
                    "group `{label}`: a member `{id}` of a physical quantity"
                ));
            }
            // The decode reads a member in place where its own gate holds,
            // which the arm it opened holds it in; a number it reads into a
            // local whatever the gates hold.
            let number = matches!(plan.value(field), Value::Scalar(_));
            if !number && field.present_if.is_none() {
                forms.push(format!(
                    "group `{label}`: a member `{id}` read whatever the gates hold, other than \
                     a number"
                ));
            }
        }
    }
    // A flag a parent may set in place of the value's is an override the
    // encode folds in last (`{codec}_overrides_t`, [`declare_overrides`]), and
    // the flag a chain's entries end on is set by the walk of the chain, in
    // place of each entry's own.
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            (ElementId::Flag { .. }, Derive::ChainContinuation) => {}
            // A number of any measure, held to what its measures say together
            // ([`derive_number`]), in a field or a flag of a carrier the value
            // always presents.
            (ElementId::Field(_) | ElementId::Flag { .. }, Derive::Number(_)) => {
                if let ElementId::Flag { field, .. } = &derived.element {
                    forms.extend(carrier_form(plan, &derived.element, field));
                }
            }
            (ElementId::Flag { field, .. }, Derive::Presence(presence)) => {
                forms.extend(carrier_form(plan, &derived.element, field));
                // Set where the elements it gates are there, each read off
                // whether the source holds it; elements the flag gates apart
                // that disagree are refused. A group presents the fields it
                // holds together, each bound off the arm the source presents
                // as an element a gate decides is.
                if presence.of.iter().any(|(gated, _)| {
                    element_of(plan, &gated.id).is_none()
                        && plan.roles.group_of(&gated.id).is_none()
                }) {
                    forms.push(format!(
                        "element '{}': a presence flag of a field the value does not present",
                        derived.element
                    ));
                }
            }
            // What the arm of the group the value holds gives the flag of the
            // disjunction that decides it (docs/adr/0065 §6 R4c).
            (ElementId::Flag { field, .. }, Derive::ArmFlag { .. }) => {
                forms.extend(carrier_form(plan, &derived.element, field));
            }
            // The tag of the arm the value holds, into its field or a flag of
            // its carrier.
            (ElementId::Field(_), Derive::VariantTag) => {}
            (ElementId::Flag { field, .. }, Derive::VariantTag) => {
                forms.extend(carrier_form(plan, &derived.element, field));
            }
            // The tag of the arm each body that fills the flag holds, read
            // off the source the value presents the body as.
            (ElementId::Flag { field, .. }, Derive::ParentTag(tag)) => {
                forms.extend(carrier_form(plan, &derived.element, field));
                if tag.terms.iter().any(|term| {
                    !matches!(
                        element_of(plan, &term.body.id).map(|e| &e.value),
                        Some(Value::Body { .. })
                    )
                }) {
                    forms.push(format!(
                        "element '{}': a tag of a body the value does not present",
                        derived.element
                    ));
                }
            }
            (element, _) => forms.push(format!(
                "element '{element}': an element the encode derives other than a number \
                 or a presence flag"
            )),
        }
    }
    for (carrier, def) in &plan.overridable {
        let element = ElementId::Flag {
            field: carrier.id.clone(),
            flag: def.name.clone(),
        };
        forms.extend(carrier_form(plan, &element, &carrier.id));
    }
    // The encode refuses what the derivations above refuse, and what a body
    // it holds refuses, which a count of its writes finds first; a gate it
    // cannot derive is held to the element it gates, tested on the carriers
    // as the encode writes them and on the inputs, and an input a held codec
    // says to what the encode hands it, a bit of a carrier or an input.
    for field in &plan.gate_checks {
        if element_of(plan, &field.id).is_none() {
            forms.push(format!(
                "field '{}': a gate held to a field the value does not present alone",
                field.id
            ));
        }
        forms.extend(gate_form(plan, field));
    }
    for check in &plan.input_checks {
        if let Against::PreviousEntry { .. } = &check.against {
            forms.push(format!(
                "element '{}': an input a chain's entry says of the entry before",
                check.at
            ));
        }
    }
    let numbers = derived_whole(plan);
    for write in &plan.program {
        match write {
            Write::Whole(field) => forms.extend(unread_form(plan, &numbers, field)),
            Write::Byte { pieces, .. } => {
                for (field, _) in pieces {
                    forms.extend(unread_form(plan, &numbers, field));
                }
            }
            Write::Tail { field, .. } => forms.extend(unread_form(plan, &numbers, field)),
            Write::Streamed { field, guard, list } => {
                forms.extend(unread_form(plan, &numbers, field));
                // A walk of each entry in turn, setting on each but the last
                // the flag a chain's entries end on and handing each whether
                // the one before it was of a kind; a list held to a law is
                // walked as any list, as its maker held its entries to the
                // law. A chain that holds a bare entry's place is not printed
                // yet.
                if list.as_ref().is_some_and(|walk| walk.slots.is_some()) {
                    forms.push(format!(
                        "field '{}': a chain whose walk holds a bare entry's place",
                        field.id
                    ));
                }
                // Written where its gate holds over the carriers the encode
                // writes and the inputs: a gate of a chain's membership reads
                // what this printer does not hold.
                if *guard == Guard::Gate {
                    forms.extend(gate_form(plan, field));
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
                    } => {}
                    BitSize::Vle { width_bits } => forms.push(format!(
                        "field '{}': a variable-length integer of {width_bits} bits",
                        field.id
                    )),
                    // Written whole from the view the source presents.
                    BitSize::Tail | BitSize::LengthRef => {}
                    // A body or a list, which the elements above name.
                    BitSize::Embed | BitSize::Repeat { .. } | BitSize::TlvChain { .. } => {}
                }
            }
            // The arm the value holds, by its codec's encode; the arms above
            // name what of it is not printed.
            Write::Variant => {}
        }
    }
    forms.dedup();
    forms
}

/// What of the carrier `carrier` of the derived flag `element` the printer
/// does not fold the flag into: it folds a flag into a number the value always
/// presents, whose other bits are the value's.
fn carrier_form(plan: &EncodePlan<'_>, element: &ElementId, carrier: &str) -> Option<String> {
    let folds = element_of(plan, carrier).is_some_and(|e| {
        e.presence == Presence::Always
            && matches!(&e.value, Value::Scalar(t) if t.int_bit_width().is_some())
    });
    (!folds).then(|| {
        format!("element '{element}': a flag of a carrier the value does not always present")
    })
}

/// The fields of `plan` whose value the encode derives whole: a number, or the
/// tag of the arm of the variant the value holds.
fn derived_whole(plan: &EncodePlan<'_>) -> BTreeSet<String> {
    plan.derivations
        .iter()
        .filter(|derived| matches!(derived.by, Derive::Number(_) | Derive::VariantTag))
        .filter_map(|derived| match &derived.element {
            ElementId::Field(id) => Some(id.clone()),
            _ => None,
        })
        .collect()
}

/// Whether the C11 value of `plan`, a plan [`unlowered`] answers `None` for,
/// holds no member `id`: the encode derives it (docs/adr/0081 §3), a number or
/// a variant's tag, the value never states it, and the decode reads it into a
/// local of its own. A number the value states while its content is absent is
/// a member, the optional value of its type.
pub(super) fn derived_field(plan: &EncodePlan<'_>, id: &str) -> bool {
    read_apart(plan).contains(id) && element_of(plan, id).is_none()
}

/// The fields of `plan` the value holds no member of its own for, which the
/// decode reads into a local of its own ([`derived_local`]): what the encode
/// derives whole, and each member of a group, which the value holds in the
/// group's arm (docs/adr/0065 §6 R4).
fn read_apart(plan: &EncodePlan<'_>) -> BTreeSet<String> {
    let mut apart = derived_whole(plan);
    for group in plan.groups() {
        apart.extend(group.fields().into_iter().map(str::to_string));
    }
    apart
}

/// The fields a codec's C11 decode reads into a local of its own,
/// [`derived_local`], and reads a length, a count or a variant's tag off where
/// it would read the member: the encode printed from its plan derives each
/// (docs/adr/0081 §3), so the value holds no member of it, or, for a number
/// the value states while its content is absent, a member the decode sets from
/// the local once it knows whether the content is there
/// ([`C11Source::decoded`]); and each member of a group, which the decode sets
/// in the arm of the group the gates it read select. Empty for a codec on the
/// template's encode, whose value holds every field, and for every language
/// whose decode binds every field to a local anyway.
#[derive(Default)]
pub(crate) struct Derived {
    /// The fields the value holds no member of its own for.
    apart: BTreeSet<String>,
    /// The members of a group the decode reads in place, each with the
    /// pointer to its place in the arm the decode opened.
    held: BTreeMap<String, String>,
}

impl Derived {
    /// The fields of `plan`, a plan [`unlowered`] answers `None` for.
    pub(super) fn of(plan: &EncodePlan<'_>) -> Self {
        Derived {
            apart: read_apart(plan),
            held: group::in_place(plan),
        }
    }

    /// Whether the value holds no member `id`.
    pub(crate) fn contains(&self, id: &str) -> bool {
        self.apart.contains(id)
    }

    /// Where `id` is a byte string, a text, a body or a list a group holds,
    /// the pointer to its place in the arm of the group the decode opened
    /// before it (`group::Group::opened`), which the decode reads it into
    /// where its gate holds: the value holds it nowhere else (docs/adr/0065
    /// §6 R4).
    pub(crate) fn held_at(&self, id: &str) -> Option<&str> {
        self.held.get(id).map(String::as_str)
    }
}

/// Whether the printer cannot read `field` as a write reads it: the value does
/// not present it, alone or in a group, and the encode does not derive it
/// whole.
fn unread_form(
    plan: &EncodePlan<'_>,
    numbers: &BTreeSet<String>,
    field: &CodecField,
) -> Option<String> {
    (element_of(plan, &field.id).is_none()
        && plan.roles.group_of(&field.id).is_none()
        && !numbers.contains(&field.id))
    .then(|| {
        format!(
            "field '{}': a field the value neither presents nor the encode derives",
            field.id
        )
    })
}

/// The element of `plan` the value presents as `id`, if it presents one.
fn element_of<'p, 'm>(plan: &'p EncodePlan<'m>, id: &str) -> Option<&'p Element<'m>> {
    plan.elements.iter().find(|e| e.field.id == id)
}

/// The C type the source presents `element` as: the type the value's struct
/// holds a number in, the view of a byte string or a text that holds its bound
/// ([`BoundedView`]), the pair a body's source is passed as, or the walk of a
/// list that holds its bound ([`BoundedList`]).
///
/// An element a gate decides, or a number the value states while its content
/// is absent, is presented as the optional value of that type: a number in the
/// type `sce/forge/optional.h` holds it in, which the value's struct holds it
/// in too, and a view, a pair or a list in the optional type declared beside
/// it.
fn presented_type(l: &LangCtx, snake: &str, element: &Element<'_>) -> String {
    let gated = element.presence.optional();
    match &element.value {
        Value::Scalar(ty) if gated => l.c11_codec_optional(ty, None).type_name,
        Value::Scalar(ty) => l.type_name(ty).into_owned(),
        Value::Bytes(_) | Value::Text(_) if gated => {
            BoundedView::of(l, snake, element.field, &element.value).optional()
        }
        Value::Bytes(_) | Value::Text(_) => {
            BoundedView::of(l, snake, element.field, &element.value).ty()
        }
        Value::Body { codec } if gated => format!("{}_source_ref_optional_t", body_prefix(codec)),
        Value::Body { codec } => format!("{}_source_ref_t", body_prefix(codec)),
        Value::List { .. } if gated => {
            BoundedList::of(l, snake, element.field, &element.value).optional()
        }
        Value::List { .. } => BoundedList::of(l, snake, element.field, &element.value).ty(),
    }
}

/// The refinement of a list or a chain (docs/adr/0065 §6 R2, R7) in C11: a
/// walk of its entries over the context it is presented with, the table each
/// entry is a source of its codec by, and the count of its entries, of a type
/// of its own for each element, made by one function that walks the list once,
/// counts it and checks the bound, and answers a status (docs/adr/0081 §2).
/// The walk is the runtime's `sce_forge_walk_t`, a begin and next pair, so an
/// array, a linked list or any store of a consumer's own presents its entries
/// where it holds them.
struct BoundedList {
    /// The prefix of its type and its maker, `{snake}_{element}_list`.
    base: String,
    /// The prefix of the symbols of the entries' codec ([`body_prefix`]).
    entry: String,
    /// How many entries the maker holds it to.
    bound: Bound,
}

impl BoundedList {
    /// The list of `field`, holding `value`, a list or a chain of the codec
    /// `snake`, presented alone or as a member of a group.
    fn of(l: &LangCtx, snake: &str, field: &CodecField, value: &Value<'_>) -> Self {
        let Value::List { codec, bound, .. } = value else {
            unreachable!("a list's refinement is of a list or a chain");
        };
        BoundedList {
            base: format!("{snake}_{}_list", l.codec_field_id(&field.id)),
            entry: body_prefix(codec),
            bound: *bound,
        }
    }

    /// Its type.
    fn ty(&self) -> String {
        format!("{}_t", self.base)
    }

    /// The optional value of it, which a source presents a list a gate
    /// decides as, made as `sce/forge/optional.h` makes one.
    fn optional(&self) -> String {
        format!("{}_optional_t", self.base)
    }

    /// The declaration of [`Self::optional`].
    fn declare_optional(&self) -> String {
        format!(
            "/* A list of {} that may be absent: absent where the gate leaves the\n \
             * element out. */\n\
             SCE_OPTIONAL_DECLARE_NAMED({}_optional, {})\n\n",
            self.ty(),
            self.base,
            self.ty()
        )
    }

    /// Its type and its maker, for the element `id` of `value`, held to the
    /// law `law` spells ([`LawText`], empty for a list held to none).
    fn declare(&self, id: &str, value: &str, law: &LawText) -> String {
        let (ty, base, entry) = (self.ty(), &self.base, &self.entry);
        let LawText {
            members,
            params,
            locals,
            checks,
            sets,
            note,
        } = law;
        let least = self.bound.least;
        let held = match self.bound.most {
            Some(most) => format!("{least} to {most} entries"),
            None => format!("{least} or more entries, the protocol stating no most"),
        };
        // An entry past the most is refused as it is met, so a walk that
        // never ends is refused too where the list has a most.
        let past = match self.bound.most {
            Some(most) => format!(
                "        if (!sce_forge_length_within_bound(count, {most})) {{\n            \
                 return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;\n        }}\n"
            ),
            None => String::new(),
        };
        let fewest = if least > 0 {
            format!(
                "    if (count < {least}u) {{\n        \
                 return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }}\n"
            )
        } else {
            String::new()
        };
        format!(
            "/* The `{id}` element of {value}: a list of {held} of {entry}_t,\n \
             * walked where it is held, which outlives the list. A list is made by\n \
             * {base}_make, which walks it once, counts its entries and refuses one\n \
             * outside the bound, so every list holds it (docs/adr/0081 §2) and the\n \
             * encode over a source writes its `count` entries, refusing nothing of\n \
             * the count. */\n\
             typedef struct {{\n    const sce_forge_walk_t *walk;\n    const void *ctx;\n    \
             const {entry}_source_t *entry;\n    size_t count;\n{members}}} {ty};\n\n\
             /* Make `*out` the list `walk` presents over `ctx`, each entry a source of\n \
             * {entry}_t by `entry`: SCE_FORGE_CODEC_OK, or the refusal of a list\n \
             * outside its bound, `*out` then as it was.{note} */\n\
             static inline sce_forge_codec_status_t {base}_make({ty} *out, \
             const sce_forge_walk_t *walk, const void *ctx, const {entry}_source_t *entry\
             {params}) {{\n    \
             size_t count = 0;\n\
             {locals}    \
             for (const void *at = walk->begin(ctx); at != NULL; at = walk->next(ctx, at)) {{\n        \
             ++count;\n\
             {past}{checks}    }}\n\
             {fewest}    out->walk = walk;\n    out->ctx = ctx;\n    out->entry = entry;\n    \
             out->count = count;\n{sets}    return SCE_FORGE_CODEC_OK;\n\
             }}\n\n"
        )
    }
}

/// What the law a list is held to (docs/adr/0065 §6 R8) adds to its
/// refinement in C11, each piece empty for a list held to none: the start of
/// each input every entry says alike (`Automaton::Uniform`), a member of the
/// list and a parameter of its maker; and the maker's hold of each entry, as
/// it walks the list once, to saying what the law asks of its place, the
/// start, or whether the entry before is of the kind the input waits for
/// (`Automaton::AfterKey`), the first entry's clear. An entry that says an
/// input in some of its values only agrees with any place where it says
/// nothing (R8b). The walk presents the same entries each time it is run
/// (`sce_forge_walk_t`), so a list its maker made holds its law for as long
/// as it is presented, and the encode over a source walks it as any list.
#[derive(Default)]
struct LawText {
    /// The members of the list's type, after its count.
    members: String,
    /// The parameters of its maker, after the entry's table.
    params: String,
    /// The maker's locals, before its walk.
    locals: String,
    /// The maker's hold of each entry `at`, inside its walk.
    checks: String,
    /// The maker's statements that set the members on `*out`.
    sets: String,
    /// What the maker's comment says of the law.
    note: String,
}

/// [`LawText`] of the list `field` of `plan`, of entries of the codec imported
/// as `codec`, held to `law`. `imports` say which inputs the entry codec says
/// in some of its values only.
fn law_text(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
    field: &CodecField,
    codec: &str,
    law: &Law,
) -> LawText {
    let mut text = LawText::default();
    if law.is_none() {
        return text;
    }
    let entry = body_prefix(codec);
    let mut updates = String::new();
    for law_input in &law.inputs {
        let name = input_name(&law_input.input);
        let said = format!("{entry}_input_{name}(entry, at)");
        let expected = match law_input.automaton {
            Automaton::Uniform => {
                text.members
                    .push_str(&format!("    uint8_t start_{name};\n"));
                text.params.push_str(&format!(", uint8_t start_{name}"));
                text.sets
                    .push_str(&format!("    out->start_{name} = start_{name};\n"));
                format!("start_{name}")
            }
            Automaton::AfterKey { id } => {
                let after = format!("_after_{name}");
                text.locals.push_str(&format!("    uint8_t {after} = 0;\n"));
                updates.push_str(&format!(
                    "        {after} = (uint8_t)({} == 0x{id:X}ull);\n",
                    entry_kind(plan, field, "entry", "at")
                ));
                after
            }
        };
        let refused = "SCE_FORGE_CODEC_PRESENT_IF_MISMATCH";
        if says_partly(imports, codec, &law_input.input) {
            text.checks.push_str(&format!(
                "        {{\n            \
                 const sce_optional_uint8_t _s = {said};\n            \
                 if (_s.present && ((_s.value != 0) != ({expected} != 0))) {{\n                \
                 return {refused};\n            }}\n        }}\n"
            ));
        } else {
            text.checks.push_str(&format!(
                "        if ((({said}) != 0) != ({expected} != 0)) {{\n            \
                 return {refused};\n        }}\n"
            ));
        }
    }
    // The kind of the entry is what the next entry's place asks of it.
    text.checks.push_str(&updates);
    text.note = "\n * Each entry says the inputs its law asks of its place (docs/adr/0065\n \
                 * §6 R8), or the list is refused with\n \
                 * SCE_FORGE_CODEC_PRESENT_IF_MISMATCH."
        .to_string();
    text
}

/// The inputs of the law of `value`, a list, that every entry says alike
/// (`Automaton::Uniform`), each as its C name: the inputs its start holds, in
/// the law's order. Empty for a list held to no law, and for any other value.
fn law_starts(value: &Value<'_>) -> Vec<String> {
    match value {
        Value::List { law, .. } => law
            .inputs
            .iter()
            .filter(|i| matches!(i.automaton, Automaton::Uniform))
            .map(|i| input_name(&i.input))
            .collect(),
        _ => Vec::new(),
    }
}

/// The member of the value's struct that holds the start of the input `input`
/// of the law of its list `list` (both C names): the value states it beside
/// the list, as the start is the value's even where the list holds no entry
/// (docs/adr/0065 §6 R8).
fn start_member(list: &str, input: &str) -> String {
    format!("{list}_start_{input}")
}

/// The members of the value's struct that hold the starts of the laws of its
/// lists ([`start_member`]), after its fields.
fn start_members(plan: &EncodePlan<'_>, l: &LangCtx) -> String {
    let mut out = String::new();
    for element in &plan.elements {
        let list = l.codec_field_id(&element.field.id);
        for input in law_starts(&element.value) {
            out.push_str(&format!(
                "    /* The `{input}` every entry of `{list}` says (docs/adr/0065 §6 R8). */\n    \
                 uint8_t {};\n",
                start_member(&list, &input)
            ));
        }
    }
    out
}

/// The kind of the entry of the chain `chain` of `plan` that the table
/// `table` presents over `at`, as a `uint64_t`: the identifier `entry-id`
/// names, read off the entry's source, less the flags `entry-id-except` leaves
/// out (`EncodePlan::entry_ids`).
fn entry_kind(plan: &EncodePlan<'_>, chain: &CodecField, table: &str, at: &str) -> String {
    let read = |field: &str| {
        format!(
            "(uint64_t){table}->{}({at})",
            filters::to_snake_case(field.to_string())
        )
    };
    match plan
        .entry_ids
        .get(&chain.id)
        .expect("a chain whose entries an input tells apart declares entry-id")
    {
        EntryId::Flag { carrier, bit, mask } => {
            format!("((({}) >> {bit}) & 0x{mask:X}ull)", read(carrier))
        }
        EntryId::Whole {
            field,
            mask: Some(mask),
        } => format!("(({}) & 0x{mask:X}ull)", read(field)),
        EntryId::Whole { field, mask: None } => format!("({})", read(field)),
    }
}

/// The refinement of a byte string or a text (docs/adr/0065 §6 R1) in C11: a
/// view, `{ data, len }`, of a type of its own for each element, made by one
/// function that checks the bound and answers a status (docs/adr/0081 §2).
struct BoundedView {
    /// The prefix of its type and its maker, `{snake}_{element}_bytes` or
    /// `{snake}_{element}_text`.
    base: String,
    /// Whether it is a text, held to UTF-8 as well as to its bound.
    text: bool,
    /// The bound the maker holds it to, in bytes.
    bound: Bound,
}

impl BoundedView {
    /// The view of `field`, holding `value`, a byte string or a text of the
    /// codec `snake`, presented alone or as a member of a group.
    fn of(l: &LangCtx, snake: &str, field: &CodecField, value: &Value<'_>) -> Self {
        let name = l.codec_field_id(&field.id);
        match value {
            Value::Bytes(bound) => BoundedView {
                base: format!("{snake}_{name}_bytes"),
                text: false,
                bound: *bound,
            },
            Value::Text(bound) => BoundedView {
                base: format!("{snake}_{name}_text"),
                text: true,
                bound: *bound,
            },
            _ => unreachable!("a view is of a byte string or a text"),
        }
    }

    /// Its type.
    fn ty(&self) -> String {
        format!("{}_t", self.base)
    }

    /// The optional value of it, which a source presents an element a gate
    /// decides as: `{ present, value }`, made by `{base}_optional_some(view)`
    /// and `{base}_optional_none()`, as `sce/forge/optional.h` makes one.
    fn optional(&self) -> String {
        format!("{}_optional_t", self.base)
    }

    /// The declaration of [`Self::optional`].
    fn declare_optional(&self) -> String {
        format!(
            "/* A view of {} that may be absent: absent where the gate leaves the\n \
             * element out. */\n\
             SCE_OPTIONAL_DECLARE_NAMED({}_optional, {})\n\n",
            self.ty(),
            self.base,
            self.ty()
        )
    }

    /// The type its data points at: a text's is `char`, as the struct holds it.
    fn data(&self) -> &'static str {
        if self.text {
            "char"
        } else {
            "uint8_t"
        }
    }

    /// Its type and its maker, for the element `id` of `value`.
    fn declare(&self, id: &str, value: &str) -> String {
        let (ty, data, base) = (self.ty(), self.data(), &self.base);
        let (what, unit) = if self.text {
            ("text", "bytes of UTF-8")
        } else {
            ("byte string", "bytes")
        };
        let least = self.bound.least;
        let held = match self.bound.most {
            Some(most) => format!("{least} to {most} {unit}"),
            None => format!("{least} or more {unit}, the protocol stating no most"),
        };
        let mut checks = length_refusal("len", least, self.bound.most);
        if self.text {
            checks.push_str(
                "    if (!sce_forge_is_valid_utf8((const uint8_t *)data, len)) {\n        \
                 return SCE_FORGE_CODEC_INVALID_UTF8;\n    }\n",
            );
        }
        format!(
            "/* The `{id}` element of {value}: a {what} of {held}, viewed where it\n \
             * is held, which outlives the view. A view is made by {base}_make, which\n \
             * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)\n \
             * and the encode over a source writes it whole, refusing nothing of it. */\n\
             typedef struct {{\n    const {data} *data;\n    size_t len;\n}} {ty};\n\n\
             /* Make `*out` a view of the `len` {unit} at `data`: SCE_FORGE_CODEC_OK, or\n \
             * the refusal of a {what} outside its bound, `*out` then as it was. */\n\
             static inline sce_forge_codec_status_t {base}_make({ty} *out, const {data} *data, \
             size_t len) {{\n\
             {checks}    out->data = data;\n    out->len = len;\n    return SCE_FORGE_CODEC_OK;\n\
             }}\n\n"
        )
    }
}

/// The statement that refuses the length `len` outside `least` and `most`
/// bytes, or nothing where every length is within them. The most is asked of
/// the runtime's `sce_forge_length_within_bound` and not compared where it is
/// written: on a target whose `size_t` is narrower than the bound, that
/// comparison is one the type decides, a warning under -Wtype-limits.
fn length_refusal(len: &str, least: u64, most: Option<u64>) -> String {
    match length_outside(len, least, most) {
        Some(outside) => format!(
            "    if ({outside}) {{\n        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }}\n"
        ),
        None => String::new(),
    }
}

/// The condition [`length_refusal`] refuses on, or `None` where it refuses no
/// length.
fn length_outside(len: &str, least: u64, most: Option<u64>) -> Option<String> {
    let mut outside = Vec::new();
    if least > 0 {
        outside.push(format!("{len} < {least}u"));
    }
    if let Some(most) = most {
        outside.push(format!("!sce_forge_length_within_bound({len}, {most})"));
    }
    (!outside.is_empty()).then(|| outside.join(" || "))
}

/// The name of the local the encode reads the element `id` into, once.
fn element_local(l: &LangCtx, id: &str) -> String {
    format!("_e_{}", l.codec_field_id(id))
}

/// The name of the local the encode derives the number `id` into, and the
/// decode reads it into: the value holds no member of it (docs/adr/0081 §3).
/// The member it stands for would be named in snake case, as every C11 member
/// of a codec is (`LangCtx::codec_field_id`).
pub(crate) fn derived_local(id: &str) -> String {
    format!("_d_{}", filters::to_snake_case(id.to_string()))
}

/// The C11 text the printer gives a codec's header.
pub(super) struct C11Source {
    /// Before the value's struct: the types of each group the value holds
    /// (docs/adr/0065 §6 R4). Empty where it holds none.
    pub(super) before_struct: String,
    /// At the end of the value's struct's members, before its variant: a
    /// member for each group, which the value holds in place of the fields the
    /// group holds, and one for the start of each input a list's law gives its
    /// entries alike ([`start_member`]). Empty where it holds neither.
    pub(super) members: String,
    /// After the value's struct and its decode, before the value's encode: the
    /// view of each byte string and text, the list of each list and chain and
    /// the walk of the value's array of it, the table of a source and the pair a
    /// parent presents a body as, the value's functions and table, the encode
    /// over any source, and the check of a value's arrays.
    pub(super) before_encode: String,
    /// The body of the value's encode, `{snake}_encode`: the check of the
    /// arrays the value and the bodies it holds keep their byte strings and
    /// texts in, which C cannot hold in a type, then the encode over the
    /// value's table.
    pub(super) encode_body: String,
    /// At the end of the streamed decode, once every field is read: each
    /// number the value states while its content is absent set from the local
    /// the decode read it into, present where none of that content was there
    /// and absent where some was, as the frame then says it of the content;
    /// and each group made of the locals its members were read into, the arm
    /// the gates the decode read select, or, for a group whose byte strings,
    /// texts, bodies or lists the decode read in place, its numbers set in the
    /// arm it opened; and the start of each list's law, what the decode handed
    /// its entries. Empty where the value states none and holds no group and
    /// no list held to a law.
    pub(super) decoded: String,
    /// Before the decode of a field, by its id: the opening of each group the
    /// decode reads some member of in place, before the first such member
    /// ([`group::Group::opened`]). Empty where no group has one.
    pub(super) before_decode: BTreeMap<String, String>,
}

/// The C11 text of the codec `plan`, a plan [`unlowered`] answers `None` for,
/// whose symbols are named after `snake` (`{snake}_t` its value's struct).
///
/// # Panics
///
/// Where the encode it prints writes no refusal the plan's totality names, or
/// one it does not (docs/adr/0065 §2): a defect of the printer.
pub(super) fn render(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    snake: &str,
) -> C11Source {
    let groups = group::groups(plan, snake);
    let printed = C11Source {
        before_struct: groups.iter().map(|g| g.declare(plan, l, snake)).collect(),
        members: groups
            .iter()
            .map(|g| format!("    {} {};\n", g.held, g.name))
            .chain(std::iter::once(start_members(plan, l)))
            .collect(),
        before_encode: before_encode(plan, l, imports, snake),
        encode_body: encode_body(plan, snake),
        decoded: decoded(plan, l, snake),
        before_decode: groups.iter().filter_map(|g| g.opened(plan, l)).collect(),
    };
    // Every refusal the totality analysis names is one the encode writes.
    plan.totality.assert_all_written(&plan.model.name);
    printed
}

/// [`C11Source::decoded`]. A number the value states measures content a gate
/// decides, and a group holds fields a gate decides, so a codec that states
/// one or holds one has gated fields, and its decode is the streamed one
/// (`fields_lay_out_by_byte`). Each statement after the first is indented as
/// the decode's own; the template indents the first.
fn decoded(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
    let mut out = Vec::new();
    for group in group::groups(plan, snake) {
        let made = group.decoded(plan);
        if !made.is_empty() {
            out.push(made.trim_end().to_string());
        }
    }
    for element in &plan.elements {
        if element.presence != Presence::Stated {
            continue;
        }
        let id = &element.field.id;
        let absent: Vec<String> = stated_contents(plan, id)
            .iter()
            .map(|content| format!("!out->{}.present", l.codec_field_id(&content.id)))
            .collect();
        let held = l.c11_codec_optional(&element.field.sce_type, None);
        let local = derived_local(id);
        out.push(format!(
            "/* `{id}` is the value's where none of the content it measures is there\n     \
             * (docs/adr/0058 §1). */\n    \
             out->{} = ({}) ? {} : {};",
            l.codec_field_id(id),
            absent.join(" && "),
            held.some(&local),
            held.none()
        ));
    }
    // The start of each input a list's law gives its entries alike: what the
    // decode handed every entry it read, the bit of a carrier it read or an
    // input it was handed (docs/adr/0065 §6 R8).
    for element in &plan.elements {
        let list = l.codec_field_id(&element.field.id);
        for law in list_laws(&plan.roles, &element.field.id) {
            let handed = match &law.against {
                InputValue::Carrier { carrier, flag } => {
                    let bit = plan
                        .model
                        .fields
                        .iter()
                        .find(|f| &f.id == carrier)
                        .and_then(|f| f.flags.iter().find(|d| &d.name == flag))
                        .map(|d| d.bit)
                        .expect("the parser found the flag a bind names");
                    format!(
                        "((((uint64_t)out->{} >> {bit}) & 1u) != 0)",
                        l.codec_field_id(carrier)
                    )
                }
                InputValue::Input { name } => format!("({} != 0)", input_name(name)),
                InputValue::PreviousEntry { .. } => continue,
            };
            out.push(format!(
                "out->{} = (uint8_t)({handed} ? 1u : 0u);",
                start_member(&list, &input_name(&law.binder.input))
            ));
        }
    }
    out.join("\n    ")
}

/// The content the number `id` the value states measures: the gated byte
/// strings, texts and bodies its derivation measures
/// (`codec_totality::stated`).
fn stated_contents<'m>(plan: &EncodePlan<'m>, id: &str) -> Vec<&'m CodecField> {
    plan.derivations
        .iter()
        .filter(|derived| matches!(&derived.element, ElementId::Field(field) if field == id))
        .filter_map(|derived| match &derived.by {
            Derive::Number(number) => Some(number),
            _ => None,
        })
        .flat_map(|number| number.measures.iter())
        .map(|measure| match &measure.of {
            Measured::Bytes { field, .. } => *field,
            Measured::Entries { list } => *list,
            Measured::Encoded { body, .. } => *body,
            _ => unreachable!("a number the value states is a length or a count of gated content"),
        })
        .collect()
}

/// [`C11Source::before_encode`].
fn before_encode(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    snake: &str,
) -> String {
    let value = format!("{snake}_t");
    let table = format!("{snake}_source_t");
    let variant = plan.model.variant.is_some();
    let mut out = String::new();
    for element in &plan.elements {
        let gated = element.presence == Presence::Gated;
        match &element.value {
            Value::Bytes(_) | Value::Text(_) => {
                let view = BoundedView::of(l, snake, element.field, &element.value);
                out.push_str(&view.declare(&element.field.id, &value));
                if gated {
                    out.push_str(&view.declare_optional());
                }
            }
            Value::List { codec, law, .. } => {
                let list = BoundedList::of(l, snake, element.field, &element.value);
                let law = law_text(plan, imports, element.field, codec, law);
                out.push_str(&list.declare(&element.field.id, &value, &law));
                if gated {
                    out.push_str(&list.declare_optional());
                }
            }
            _ => {}
        }
    }
    // The views and lists each group's members are presented as, and the
    // group as a source presents it where that is not as the value holds it.
    let groups = group::groups(plan, snake);
    for g in &groups {
        out.push_str(&g.declare_presented(plan, l, snake));
    }
    let arms = Arms::of(plan);
    if variant {
        out.push_str(&declare_body_ref(plan, l, snake, &arms));
    }
    out.push_str(&format!(
        "/* The elements a value of {value} presents, which its encode reads\n \
         * (docs/adr/0081 §2): one function for each, handed the context the\n \
         * table is passed with. A value of {value} is a source by the table\n \
         * {snake}_value_source() answers, its context the value; any other type\n \
         * that presents the elements is one by a table of its own. The encode\n \
         * derives every other element of the frame. */\n\
         typedef struct {{\n"
    ));
    if plan.elements.is_empty() && groups.is_empty() && !variant {
        out.push_str(
            "    /* A C struct declares at least one member; the codec presents no\n     \
             * element, so the table has no function. */\n    \
             char _reserved;\n",
        );
    }
    for element in &plan.elements {
        out.push_str(&format!(
            "    {} (*{})(const void *ctx);\n",
            presented_type(l, snake, element),
            l.codec_field_id(&element.field.id)
        ));
    }
    // Each group, as a source presents it, after the elements, as the value's
    // struct holds it after its fields.
    for g in &groups {
        out.push_str(&format!(
            "    {} (*{})(const void *ctx);\n",
            g.presented, g.name
        ));
    }
    // The arm of the variant, after the elements, as the value's struct holds
    // it after its fields.
    if variant {
        out.push_str(&format!(
            "    {snake}_body_ref_t (*{BODY})(const void *ctx);\n"
        ));
    }
    out.push_str(&format!("}} {table};\n\n"));
    // How a parent presents a body of this codec: the table of its source and
    // the context the table is handed, two pointers, so a body is presented
    // where it is held and with no copy; and the optional value of the pair,
    // which a parent presents a body a gate decides as.
    out.push_str(&format!(
        "/* A source of {value}: the table of its elements and the context the\n \
         * table is handed, as a codec that holds one presents it to its encode. */\n\
         typedef struct {{\n    const {table} *table;\n    const void *ctx;\n}} {snake}_source_ref_t;\n\n\
         /* A source of {value} that may be absent. */\n\
         SCE_OPTIONAL_DECLARE_NAMED({snake}_source_ref_optional, {snake}_source_ref_t)\n\n"
    ));

    // The value's functions, each reading its element off the value the
    // context points at: a number as the struct holds it, a byte string or a
    // text as a view of the array that holds it and the length beside it,
    // within the bound the value's encode holds the array to. One a gate
    // decides is read off the optional value the struct holds it in: a number
    // as it is, and a view of the bytes and their length where it is present.
    for element in &plan.elements {
        let name = l.codec_field_id(&element.field.id);
        let ty = presented_type(l, snake, element);
        let gated = element.presence == Presence::Gated;
        let body = match &element.value {
            Value::Bytes(_) | Value::Text(_) if gated => {
                let view = BoundedView::of(l, snake, element.field, &element.value);
                let (view_ty, base) = (view.ty(), &view.base);
                // The struct holds a text's UTF-8 as bytes when a gate decides it.
                let data = if view.text {
                    format!("(const char *)self->{name}.value.data")
                } else {
                    format!("self->{name}.value.data")
                };
                format!(
                    "    const {value} *self = (const {value} *)ctx;\n    \
                     if (!self->{name}.present) {{\n        \
                     return {base}_optional_none();\n    }}\n    \
                     {view_ty} view;\n    \
                     view.data = {data};\n    \
                     view.len = self->{name}.value.len;\n    \
                     return {base}_optional_some(view);\n"
                )
            }
            Value::Bytes(_) | Value::Text(_) => format!(
                "    const {value} *self = (const {value} *)ctx;\n    \
                 {ty} view;\n    \
                 view.data = self->{name};\n    \
                 view.len = self->{name}_len;\n    \
                 return view;\n"
            ),
            // The body the struct holds, as a source of its codec by the table
            // that presents a value of it.
            Value::Body { codec } => {
                let prefix = body_prefix(codec);
                let (held, absent) = if gated {
                    (
                        format!("self->{name}.value"),
                        format!(
                            "    if (!self->{name}.present) {{\n        \
                             return {prefix}_source_ref_optional_none();\n    }}\n"
                        ),
                    )
                } else {
                    (format!("self->{name}"), String::new())
                };
                let ret = if gated {
                    format!("{prefix}_source_ref_optional_some(body)")
                } else {
                    "body".to_string()
                };
                format!(
                    "    const {value} *self = (const {value} *)ctx;\n\
                     {absent}    {prefix}_source_ref_t body;\n    \
                     body.table = {prefix}_value_source();\n    \
                     body.ctx = &{held};\n    \
                     return {ret};\n"
                )
            }
            // The entries the struct holds in its array, walked by the value's
            // own begin and next over it, each a source of its codec by the
            // table that presents a value of it, and counted by the length
            // beside the array, within the bound the value's check holds it
            // to.
            Value::List { .. } => {
                let list = BoundedList::of(l, snake, element.field, &element.value);
                let (list_ty, base, entry) = (list.ty(), &list.base, &list.entry);
                let (data, len) = if gated {
                    (
                        format!("self->{name}.value.data"),
                        format!("self->{name}.value.len"),
                    )
                } else {
                    (format!("self->{name}"), format!("self->{name}_len"))
                };
                out.push_str(&format!(
                    "/* The first entry of the `{name}` of the {value} at `ctx`, or NULL where\n \
                     * it holds none. */\n\
                     static inline const void *{snake}_value_{name}_begin(const void *ctx) {{\n    \
                     const {value} *self = (const {value} *)ctx;\n    \
                     return {len} > 0u ? (const void *)&{data}[0] : NULL;\n\
                     }}\n\n\
                     /* The entry after `entry` of the `{name}` of the {value} at `ctx`, or\n \
                     * NULL where `entry` is its last. */\n\
                     static inline const void *{snake}_value_{name}_next(const void *ctx, \
                     const void *entry) {{\n    \
                     const {value} *self = (const {value} *)ctx;\n    \
                     const {entry}_t *at = (const {entry}_t *)entry + 1;\n    \
                     return at < {data} + {len} ? (const void *)at : NULL;\n\
                     }}\n\n"
                ));
                let (absent, ret) = if gated {
                    (
                        format!(
                            "    if (!self->{name}.present) {{\n        \
                             return {base}_optional_none();\n    }}\n"
                        ),
                        format!("{base}_optional_some(list)"),
                    )
                } else {
                    (String::new(), "list".to_string())
                };
                // The start of each input its law gives every entry alike,
                // which the value holds beside the list (docs/adr/0065 §6 R8).
                let starts: String = law_starts(&element.value)
                    .iter()
                    .map(|input| {
                        format!(
                            "    list.start_{input} = self->{};\n",
                            start_member(&name, input)
                        )
                    })
                    .collect();
                format!(
                    "    static const sce_forge_walk_t walk = {{\n        \
                     {snake}_value_{name}_begin,\n        \
                     {snake}_value_{name}_next,\n    }};\n    \
                     const {value} *self = (const {value} *)ctx;\n\
                     {absent}    {list_ty} list;\n    \
                     list.walk = &walk;\n    \
                     list.ctx = ctx;\n    \
                     list.entry = {entry}_value_source();\n    \
                     list.count = {len};\n\
                     {starts}    \
                     return {ret};\n"
                )
            }
            _ => format!("    return ((const {value} *)ctx)->{name};\n"),
        };
        out.push_str(&format!(
            "/* The `{name}` of the {value} at `ctx`. */\n\
             static inline {ty} {snake}_value_{name}(const void *ctx) {{\n\
             {body}\
             }}\n\n"
        ));
    }
    // Each group, as the struct holds it, each member presented where the
    // struct holds it.
    for g in &groups {
        out.push_str(&g.reader(plan, l, snake));
    }
    // The arm the struct holds, its body a source of the arm's codec by the
    // table that presents a value of it, pointed at the body the union holds.
    if variant {
        let cases = arms.switch(&format!("self->{BODY}.kind"), &arms.all(), |arm| {
            let (table, ctx) = arm_source("ref", arm);
            format!(
                "            {table} = {}_value_source();\n            \
                 {ctx} = &self->{BODY}.arm.{};\n",
                arm.prefix, arm.member
            )
        });
        out.push_str(&format!(
            "/* The arm of the variant of the {value} at `ctx`: its kind, the tag the\n \
             * catch-all holds, and its body as a source of the arm's codec. */\n\
             static inline {snake}_body_ref_t {snake}_value_{BODY}(const void *ctx) {{\n    \
             const {value} *self = (const {value} *)ctx;\n    \
             {snake}_body_ref_t ref;\n    \
             memset(&ref, 0, sizeof ref);\n    \
             ref.kind = self->{BODY}.kind;\n    \
             ref.default_tag = self->{BODY}.default_tag;\n\
             {cases}    return ref;\n\
             }}\n\n"
        ));
    }
    let mut entries: Vec<String> = plan
        .elements
        .iter()
        .map(|element| format!("{snake}_value_{}", l.codec_field_id(&element.field.id)))
        .collect();
    entries.extend(groups.iter().map(|g| format!("{snake}_value_{}", g.name)));
    if variant {
        entries.push(format!("{snake}_value_{BODY}"));
    }
    let entries = if entries.is_empty() {
        "0".to_string()
    } else {
        entries.join(",\n        ")
    };
    out.push_str(&format!(
        "/* The table that presents a value of {value} as a source of it, its\n \
         * context a `const {value} *`. One table for every value, in read-only\n \
         * memory. */\n\
         static inline const {table} *{snake}_value_source(void) {{\n    \
         static const {table} table = {{\n        {entries},\n    }};\n    \
         return &table;\n\
         }}\n\n"
    ));
    out.push_str(&arm_inputs(plan, imports, snake, &arms));
    out.push_str(&encode_source(plan, l, snake));
    out.push('\n');
    out.push_str(&check_value(plan, l, snake));
    out
}

/// `{snake}_body_ref_t`: the arm of the variant a source presents, as the
/// value's struct holds its own (`{snake}_variant_t`), its kind and the tag the
/// catch-all holds beside the source of the arm's body in a member of the same
/// name, each arm a build feature decides inside the guard of its macro. A
/// body is presented where it is held, by two pointers, with no copy.
fn declare_body_ref(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str, arms: &Arms) -> String {
    let tag = l.type_name(&crate::forge::codec_totality::tag_type(plan.model));
    let mut members = String::new();
    for (i, arm) in arms.arms.iter().enumerate() {
        members.push_str(&arms.guarded(
            i,
            &format!("        {}_source_ref_t {};\n", arm.prefix, arm.member),
        ));
    }
    format!(
        "/* The arm of the variant a source of {snake}_t presents (docs/adr/0081\n \
         * §2): which arm it is, the tag the catch-all holds, and the arm's body,\n \
         * a source of the arm's codec in the member `kind` names. */\n\
         typedef struct {{\n    {} kind;\n    {tag} default_tag;  /* the catch-all's */\n    \
         union {{\n{members}    }} arm;\n}} {snake}_body_ref_t;\n\n",
        kind_type(snake)
    )
}

/// The function `{snake}_arm_input_<name>` of each flag input the arms of the
/// variant say (docs/adr/0065 §6 R4d, R4e): what the arm a source presents
/// says, through its codec's own function of the input over the arm's body. A
/// `uint8_t` where every arm says it; where some arm does not read it, the
/// optional value, absent for such an arm, as an arm that says the input in
/// some of its values only answers it.
fn arm_inputs(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
    snake: &str,
    arms: &Arms,
) -> String {
    let mut out = String::new();
    for said in &plan.said {
        let (bound, partly) = match plan.roles.inputs.get(&said.input.name) {
            Some(InputSource::EveryArm { arms: bound }) => (bound, false),
            Some(InputSource::SomeArms { arms: bound }) => (bound, true),
            _ => continue,
        };
        let name = input_name(&said.input.name);
        let mut cases = String::new();
        for (i, arm) in arms.arms.iter().enumerate() {
            let Some(input) = bound.get(&arm.alias) else {
                continue;
            };
            let (table, ctx) = arm_source("arm", arm);
            let read = format!("{}_input_{}({table}, {ctx})", arm.prefix, input_name(input));
            let answer = if partly && !says_partly(imports, &arm.alias, input) {
                format!("sce_optional_uint8_some({read})")
            } else {
                read
            };
            cases.push_str(&arms.guarded(
                i,
                &format!("        case {}:\n            return {answer};\n", arm.kind),
            ));
        }
        let cases = format!(
            "    switch (arm.kind) {{\n{cases}        default:\n            break;\n    }}\n"
        );
        let (ty, nothing) = if partly {
            ("sce_optional_uint8_t", "sce_optional_uint8_none()")
        } else {
            ("uint8_t", "0")
        };
        out.push_str(&format!(
            "/* The flag input `{}` the arm `arm` presents says (docs/adr/0065 §6\n \
             * R4d), through the arm's codec's own function of it{}. */\n\
             static inline {ty} {snake}_arm_input_{name}({snake}_body_ref_t arm) {{\n\
             {cases}    return {nothing};\n\
             }}\n\n",
            said.input.name,
            if partly {
                "; nothing where\n * the arm does not read it (R4e)"
            } else {
                ""
            }
        ));
    }
    out
}

/// The encode of `{snake}_t` over any source of it: each element read once off
/// the source into a local, what the encode derives computed from them and
/// any value the encoding cannot carry refused, then the writes in wire order.
///
/// Where a body the value holds can refuse, a refusal of the body's own is
/// met only as the body is written, so the encode writes the frame over a
/// counting sink first (docs/adr/0081 §1, the program), which finds it before
/// the first byte: `{snake}_encode_source` counts by `{snake}_write_source`,
/// the writes alone, and then writes by it.
fn encode_source(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
    let refuses = if plan.totality.is_total() {
        "It refuses no value: it returns SCE_FORGE_CODEC_OK, or\n \
         * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left."
    } else {
        "A value the encoding cannot carry is refused by the\n \
         * status that names why before the first byte is written (docs/adr/0065\n \
         * §2); any other returns SCE_FORGE_CODEC_OK, or\n \
         * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left."
    };
    let children_refuse = plan
        .totality
        .refusals()
        .iter()
        .any(|r| r.kind == RefusalKind::Child);
    // A codec whose flags a parent may set is encoded by
    // `{snake}_encode_source_overriding`, handed the flags the parent sets
    // (`{snake}_overrides_t`), and `{snake}_encode_source` is that encode
    // handed none, so every other caller calls it as it calls any codec's.
    // The flag inputs the value does not say are handed to the encode after
    // the sink (docs/adr/0065 §6 R4a), one byte each, in the order the codec
    // declares them, then the overrides.
    let overriding = !plan.overridable.is_empty();
    let mut params = given_params(plan);
    let mut args = given_args(plan);
    let encode = if overriding {
        params.push_str(&format!(", {snake}_overrides_t overrides"));
        args.push_str(", overrides");
        "encode_source_overriding"
    } else {
        "encode_source"
    };
    let signature = |name: &str| {
        format!(
            "static inline sce_forge_codec_status_t {snake}_{name}(const {snake}_source_t *src, \
             const void *ctx, sce_forge_writer_t *w{params}) {{\n"
        )
    };
    let mut out = String::new();
    if overriding {
        out.push_str(&declare_overrides(plan, l, snake));
    }
    if children_refuse {
        // A body's own refusal passes on through each write of it, which the
        // count before the writes finds first.
        for r in plan.totality.refusals() {
            if r.kind == RefusalKind::Child {
                plan.totality.require(RefusalKind::Child, &r.element);
            }
        }
        out.push_str(&format!(
            "/* {snake}_{encode} without the count that finds a refusal of a body\n \
             * the source holds before the first byte is written. */\n\
             {}",
            signature("write_source")
        ));
    } else {
        out.push_str(&format!(
            "/* Write the source `src` presents over `ctx` into `w`, each element read\n \
             * once and written where the codec's encode plan lays it out\n \
             * (docs/adr/0081). {refuses} */\n\
             {}",
            signature(encode)
        ));
    }
    let variant = plan.model.variant.is_some();
    let groups = group::groups(plan, snake);
    if plan.elements.is_empty() && groups.is_empty() && !variant {
        out.push_str("    (void)src;\n    (void)ctx;\n");
    }
    if plan.program.is_empty() {
        out.push_str("    (void)w;\n");
    }
    for element in &plan.elements {
        out.push_str(&format!(
            "    const {} {} = src->{}(ctx);\n",
            presented_type(l, snake, element),
            element_local(l, &element.field.id),
            l.codec_field_id(&element.field.id)
        ));
    }
    // Each group, read once, and each of its members bound off the arm it
    // holds to the optional value an element a gate decides is read into, so
    // every statement below reads a member as it reads such an element
    // (docs/adr/0065 §6 R4).
    for g in &groups {
        let held = element_local(l, &g.name);
        out.push_str(&format!(
            "    const {} {held} = src->{}(ctx);\n",
            g.presented, g.name
        ));
        for id in g.group.fields() {
            out.push_str(&format!(
                "    const {} {} = {};\n",
                g.member_optional(plan, l, snake, id),
                element_local(l, id),
                g.member_read(plan, l, snake, id, &held)
            ));
        }
    }
    // The arm of the variant, read once as the elements are.
    let arms = Arms::of(plan);
    if variant {
        out.push_str(&format!(
            "    const {snake}_body_ref_t {} = src->{BODY}(ctx);\n",
            element_local(l, BODY)
        ));
    }
    // An input is read where a gate, a check or a held codec needs it, which
    // some layouts never do.
    for input in &plan.given {
        out.push_str(&format!("    (void){};\n", input_name(&input.name)));
    }
    // Each input the value says that a gate or a check of the codec reads,
    // read off the elements under the name the gate reads it by; one no gate
    // reads is left to the parent that asks for it, through the input's own
    // function ([`said_inputs`]).
    for said in plan
        .said
        .iter()
        .filter(|said| encode_reads(plan, &said.input.name))
    {
        let name = input_name(&said.input.name);
        out.push_str(&format!(
            "    const uint8_t {name} = {};\n    (void){name};\n",
            said_input(plan, l, snake, &said.input.name, Read::Encode)
        ));
    }
    // What the encode derives (docs/adr/0081 §3), each refusal of it before
    // the first byte: a number from what it measures ([`derive_number`]),
    // held whole in a field as the local `_d_<field>`; and the flags of a
    // carrier, each folded into it: a presence flag set exactly where the
    // elements it gates are there (or are not, for one gated on the flag's
    // clear state), refused where two of them disagree, and a number its
    // bit-range holds.
    let mut folds: BTreeMap<&str, Vec<Fold>> = BTreeMap::new();
    // A number of encoded lengths is counted once the carriers its bodies
    // may be handed are as the encode writes them, below; such a number goes
    // into a field, never into a flag a carrier holds.
    let mut encoded: Vec<(&ElementId, &Number<'_>)> = Vec::new();
    let mut counted = BTreeSet::new();
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            (element, Derive::Number(number))
                if number
                    .measures
                    .iter()
                    .any(|m| matches!(m.of, Measured::Encoded { .. })) =>
            {
                encoded.push((element, number));
            }
            (ElementId::Flag { field, flag }, Derive::Presence(presence)) => {
                let tests: Vec<String> = presence
                    .of
                    .iter()
                    .map(|(gated, when_clear)| {
                        let there = format!("{}.present", element_local(l, &gated.id));
                        if *when_clear {
                            format!("!{there}")
                        } else {
                            there
                        }
                    })
                    .collect();
                let (first, others) = tests.split_first().expect("a presence flag gates a field");
                // Elements the flag gates apart that disagree have no frame;
                // a group's fields are there together and cannot.
                if !presence.together {
                    for other in others {
                        out.push_str(&refuse_if(
                            &format!("({first}) != ({other})"),
                            &refusal(
                                plan,
                                RefusalKind::InconsistentSource,
                                &derived.element.to_string(),
                            ),
                        ));
                    }
                }
                folds.entry(field.as_str()).or_default().push(Fold {
                    flag: flag.clone(),
                    named: derived.element.to_string(),
                    bits: format!("(uint64_t)({first})"),
                    count: None,
                    present: None,
                });
            }
            (element, Derive::Number(number)) => {
                let measures: Vec<MeasureText> = number
                    .measures
                    .iter()
                    .map(|measure| {
                        measure_text(plan, l, element, measure, None, &mut counted, &mut out)
                    })
                    .collect();
                if let Some((carrier, fold)) =
                    derive_number(plan, l, element, &number.into, &measures, &mut out)
                {
                    folds.entry(carrier).or_default().push(fold);
                }
            }
            // The tag of the arm the value holds: the one the document gives a
            // known arm, and the one the catch-all holds, into the tag's field
            // whole or into its flag, masked to the flag's bits.
            (element, Derive::VariantTag) => {
                let held = element_local(l, BODY);
                let cases =
                    arms.switch(&format!("{held}.kind"), &arms.all(), |arm| match arm.tag {
                        Some(tag) => format!("            _t_body = 0x{tag:X}ull;\n"),
                        None => format!("            _t_body = (uint64_t){held}.default_tag;\n"),
                    });
                out.push_str(&format!("    uint64_t _t_body = 0;\n{cases}"));
                match element {
                    ElementId::Field(id) => {
                        let field = plan
                            .model
                            .fields
                            .iter()
                            .find(|f| f.id == *id)
                            .expect("a tag is derived into a field of the codec");
                        let ty = l.type_name(&field.sce_type);
                        out.push_str(&format!(
                            "    const {ty} {} = ({ty})_t_body;\n",
                            derived_local(id)
                        ));
                    }
                    ElementId::Flag { field, flag } => {
                        let mask = flag_mask(plan, field, flag);
                        folds.entry(field.as_str()).or_default().push(Fold {
                            flag: flag.clone(),
                            named: element.to_string(),
                            bits: format!("(_t_body & 0x{mask:X}ull)"),
                            count: None,
                            present: None,
                        });
                    }
                    ElementId::Variant => {
                        unreachable!("a variant's tag is derived into a field or a flag")
                    }
                }
            }
            // The tag of the arm each body that fills the flag holds, masked
            // to the flag's bits, the catch-all's the tag it holds; nothing
            // from a body a gate leaves out, and where every body is gated
            // and none is there, the bits the value presents.
            (ElementId::Flag { field, flag }, Derive::ParentTag(tag)) => {
                let local = format!("_p_{}", override_member(l, field, flag));
                out.push_str(&format!("    uint64_t {local} = 0;\n"));
                out.push_str(&parent_tag(plan, l, tag, &local));
                folds.entry(field.as_str()).or_default().push(Fold {
                    flag: flag.clone(),
                    named: derived.element.to_string(),
                    bits: local,
                    count: None,
                    present: parent_tag_said(plan, l, tag),
                });
            }
            // A flag of a disjunction that decides a group: what the arm of the
            // group the source presents gives it, or what it has while the
            // value holds no arm (docs/adr/0065 §6 R4c).
            (ElementId::Flag { field, flag }, Derive::ArmFlag { group, gate }) => {
                let g = &groups[*group];
                let value = g.gate_value(*gate, &element_local(l, &g.name));
                folds.entry(field.as_str()).or_default().push(Fold {
                    flag: flag.clone(),
                    named: derived.element.to_string(),
                    bits: format!("(uint64_t){value}"),
                    count: None,
                    present: None,
                });
            }
            // Set by the walk of the chain that holds the entry, not here.
            (ElementId::Flag { .. }, Derive::ChainContinuation) => {}
            _ => unreachable!(
                "`unlowered` prints a derived number, a presence flag, a group's flag, a \
                 variant's tag, a parent's tag or a chain's flag only"
            ),
        }
    }
    // A flag a parent sets in place of the one the value presents (the flag a
    // chain's entries end on), folded last: what the parent says is what the
    // wire holds, and where it says nothing the value's own bit stays.
    for (carrier, def) in &plan.overridable {
        let member = override_member(l, &carrier.id, &def.name);
        folds.entry(carrier.id.as_str()).or_default().push(Fold {
            flag: def.name.clone(),
            named: format!("{}.{}", carrier.id, def.name),
            bits: format!("(uint64_t)overrides.{member}.value"),
            count: None,
            present: Some(format!("overrides.{member}.present")),
        });
    }
    // Each carrier as the source presents it, each derived bit-range replaced
    // in turn, the bits it holds no wider than the grammar bounds them.
    for (carrier_id, flags) in &folds {
        let carrier = plan
            .model
            .fields
            .iter()
            .find(|f| f.id == *carrier_id)
            .expect("a derived flag is a flag of a carrier of the codec");
        let ty = l.type_name(&carrier.sce_type);
        let name = l.codec_field_id(&carrier.id);
        let bits = format!("_f_{name}");
        out.push_str(&format!(
            "    uint64_t {bits} = (uint64_t){};\n",
            element_local(l, &carrier.id)
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
            // A number past what its bits hold has no frame (docs/adr/0065
            // §5), refused where the analysis says it may be.
            let mut set = String::new();
            if let Some(count) = &fold.count {
                if plan
                    .totality
                    .refuses(RefusalKind::LengthOutOfRange, &fold.named)
                {
                    set.push_str(&refuse_if(
                        &format!("!sce_forge_length_within_bound((size_t)({count}), {width_mask})"),
                        &refusal(plan, RefusalKind::LengthOutOfRange, &fold.named),
                    ));
                }
            }
            set.push_str(&format!(
                "    {bits} = ({bits} & ~0x{mask:X}ull) | {placed};\n"
            ));
            // Where nothing it measures is there, the carrier keeps the bits
            // the value presents.
            out.push_str(&match &fold.present {
                None => set,
                Some(there) => guarded(there, &set),
            });
        }
        out.push_str(&format!("    const {ty} _c_{name} = ({ty}){bits};\n"));
    }
    // The value a field is written as: the number or the tag the encode
    // derived, the carrier with its derived flags, the present value of an
    // element a gate decides (read only where it is tested present), or what
    // the source presents.
    let numbers = derived_whole(plan);
    let read = |field: &CodecField| {
        if numbers.contains(&field.id) {
            derived_local(&field.id)
        } else if folds.contains_key(field.id.as_str()) {
            format!("_c_{}", l.codec_field_id(&field.id))
        } else if plan.presence(field) == Presence::Gated {
            format!("{}.value", element_local(l, &field.id))
        } else {
            element_local(l, &field.id)
        }
    };
    // Whether the gate of a field holds over the carriers the encode writes
    // and the inputs, and what a held codec is handed.
    let gate_holds = |field: &CodecField| gate_test(plan, field, &read);
    let handed: &dyn Fn(&str) -> String = &|alias| handed_args(plan, alias, &read);

    // What a value is held to before the first byte is written (docs/adr/0065
    // §2), so a refusal leaves the writer at its start: each input a held
    // codec says, to what this codec hands it, and each gate the encode
    // cannot derive the presence from, to the element it gates.
    for check in &plan.input_checks {
        out.push_str(&input_check(plan, l, check, &read));
    }
    for field in &plan.gate_checks {
        let present = format!("{}.present", element_local(l, &field.id));
        out.push_str(&refuse_if(
            &format!("({}) != {present}", gate_holds(field)),
            &refusal(plan, RefusalKind::PresentIfMismatch, &field.id),
        ));
    }
    // How many bytes each body encodes to, counted by its encode, handed what
    // it is handed where it is written, over a counting sink.
    for (element, number) in encoded {
        let measures: Vec<MeasureText> = number
            .measures
            .iter()
            .map(|measure| {
                measure_text(
                    plan,
                    l,
                    element,
                    measure,
                    Some(handed),
                    &mut counted,
                    &mut out,
                )
            })
            .collect();
        if derive_number(plan, l, element, &number.into, &measures, &mut out).is_some() {
            unreachable!("a number of encoded lengths goes into a field");
        }
    }
    let write =
        |byte: &str| format!("    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, {byte}));\n");
    // A byte string or a text, whole, from the view the source presents: a
    // text's view points at `char`, which the writer takes as its bytes.
    let view_bytes = |field: &CodecField| {
        let local = read(field);
        let data = if field.sce_type == SceType::String {
            format!("(const uint8_t *){local}.data")
        } else {
            format!("{local}.data")
        };
        format!("    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, {data}, {local}.len));\n")
    };
    let endian = plan.model.default_endian;
    for step in &plan.program {
        match step {
            Write::Whole(field) => {
                for byte in encode_whole_field_read_as(field, endian, &read(field), l) {
                    out.push_str(&write(&byte));
                }
            }
            Write::Byte { pieces, .. } => {
                out.push_str(&write(&encode_byte_read_by(pieces, l, &read)));
            }
            Write::Streamed { field, guard, .. } => {
                let mut body = String::new();
                match &field.bit_size {
                    BitSize::Fixed { .. } => {
                        for byte in encode_whole_field_read_as(field, endian, &read(field), l) {
                            body.push_str(&write(&byte));
                        }
                    }
                    // The runtime's writer, as the template's encode calls it.
                    BitSize::Vle { width_bits } => {
                        let value = l.codec_carrier_expr(&field.sce_type, &read(field));
                        body.push_str(&vle_encode_block(&value, *width_bits, l.lang));
                        body.push('\n');
                    }
                    // Its length, where the frame carries one, a field of its
                    // own the program writes before it.
                    BitSize::Tail | BitSize::LengthRef => body.push_str(&view_bytes(field)),
                    // A body, by its codec's encode over the source the value
                    // presents it as, alone or as a member of a group.
                    BitSize::Embed => {
                        let Value::Body { codec } = plan.value(field) else {
                            unreachable!("an embedded field is a body the value presents");
                        };
                        let held = read(field);
                        body.push_str(&format!(
                            "    SCE_FORGE_TRY_WRITE({}_encode_source({held}.table, {held}.ctx, w{}));\n",
                            body_prefix(codec),
                            handed(codec)
                        ));
                    }
                    // A list or a chain that ends with its scope, its entries
                    // walked from the first, each written by its codec's
                    // encode over the source the list presents it as; no
                    // more than the count the list's maker found, which a
                    // count before it says.
                    BitSize::Repeat { .. } | BitSize::TlvChain { .. } => {
                        let Value::List { codec, .. } = plan.value(field) else {
                            unreachable!("a list or a chain is an element the value presents");
                        };
                        let held = read(field);
                        let entry = body_prefix(codec);
                        let args = handed(codec);
                        let walk = match step {
                            Write::Streamed {
                                list: Some(walk), ..
                            } => Some(walk),
                            _ => None,
                        };
                        let ends = walk.and_then(|walk| walk.ends.as_ref());
                        // A chain hands each entry, for an input nothing binds,
                        // whether the entry before it was of the kind the
                        // input waits for, the first entry's clear
                        // (docs/adr/0065 §6 R8b), as [`handed_args`] names it.
                        let mut before = String::new();
                        let mut after = String::new();
                        for (input, id) in walk
                            .map(|walk| walk.previous.as_slice())
                            .unwrap_or_default()
                        {
                            let local = previous_local(input);
                            before.push_str(&format!("        uint8_t {local} = 0;\n"));
                            after.push_str(&format!(
                                "            {local} = (uint8_t)({} == 0x{id:X}ull);\n",
                                entry_kind(plan, field, &format!("{held}.entry"), "_at")
                            ));
                        }
                        // A chain sets on each entry but the last the flag its
                        // entries end on, in place of the one the entry
                        // presents (docs/adr/0058 §1), by the entry's codec's
                        // overrides.
                        let write = match ends {
                            Some((carrier, flag)) => format!(
                                "            {entry}_overrides_t _o = {entry}_overrides_none();\n            \
                                 _o.{member} = sce_optional_bool_some(_i + 1u < {held}.count);\n            \
                                 SCE_FORGE_TRY_WRITE({entry}_encode_source_overriding({held}.entry, \
                                 _at, w{args}, _o));\n",
                                member = override_member(l, carrier, flag)
                            ),
                            None => format!(
                                "            SCE_FORGE_TRY_WRITE({entry}_encode_source({held}.entry, \
                                 _at, w{args}));\n"
                            ),
                        };
                        body.push_str(&format!(
                            "    {{\n        \
                             const void *_at = {held}.walk->begin({held}.ctx);\n\
                             {before}        \
                             for (size_t _i = 0; _i < {held}.count && _at != NULL; ++_i) {{\n\
                             {write}{after}            \
                             _at = {held}.walk->next({held}.ctx, _at);\n        \
                             }}\n    \
                             }}\n"
                        ));
                    }
                }
                out.push_str(&match guard {
                    Guard::Always => body,
                    // Where the value holds it.
                    Guard::Present => {
                        guarded(&format!("{}.present", element_local(l, &field.id)), &body)
                    }
                    // Derived whole and gated: where its gate holds, as the
                    // carriers the encode writes say.
                    Guard::Gate => guarded(&gate_holds(field), &body),
                });
            }
            // A byte string or a text of a positional layout, whole, after
            // zeros up to its offset where the layout aligns it.
            Write::Tail { field, pad } => {
                if *pad {
                    out.push_str(&format!(
                        "    while (sce_forge_writer_position(w) < {}) {{\n        \
                         SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, 0));\n    }}\n",
                        field.byte_offset
                    ));
                }
                out.push_str(&view_bytes(field));
            }
            // The arm the value holds, by its codec's encode over the source
            // of its body, handed what its import binds.
            Write::Variant => {
                let held = element_local(l, BODY);
                out.push_str(&arms.switch(&format!("{held}.kind"), &arms.all(), |arm| {
                    let (table, ctx) = arm_source(&held, arm);
                    format!(
                        "            SCE_FORGE_TRY_WRITE({}_encode_source({table}, {ctx}, w{}));\n",
                        arm.prefix,
                        handed(&arm.alias)
                    )
                }));
            }
        }
    }
    out.push_str("    return SCE_FORGE_CODEC_OK;\n}\n");
    if children_refuse {
        out.push_str(&format!(
            "\n/* Write the source `src` presents over `ctx` into `w`, each element read\n \
             * once and written where the codec's encode plan lays it out\n \
             * (docs/adr/0081). {refuses} The frame is written over a\n \
             * counting sink first, which stores nothing, so a refusal of a body the\n \
             * source holds is met before the first byte. */\n\
             {}    {{\n        \
             sce_forge_writer_t _count = sce_forge_writer_init_counting();\n        \
             SCE_FORGE_TRY_WRITE({snake}_write_source(src, ctx, &_count{args}));\n    \
             }}\n    \
             return {snake}_write_source(src, ctx, w{args});\n\
             }}\n",
            signature(encode)
        ));
    }
    if overriding {
        let (given, passed) = (given_params(plan), given_args(plan));
        out.push_str(&format!(
            "\n/* {snake}_encode_source_overriding with no flag set in place of the\n \
             * value's: the encode every caller but a parent that sets one calls. */\n\
             static inline sce_forge_codec_status_t {snake}_encode_source(\
             const {snake}_source_t *src, const void *ctx, sce_forge_writer_t *w{given}) {{\n    \
             return {snake}_encode_source_overriding(src, ctx, w{passed}, \
             {snake}_overrides_none());\n\
             }}\n"
        ));
    }
    out.push_str(&said_inputs(plan, l, snake));
    out
}

/// The parameters of an encode over a source that the flag inputs the value
/// does not say take, after the sink: `, uint8_t <name>` each, in the order
/// the codec declares them, as the decode names them.
fn given_params(plan: &EncodePlan<'_>) -> String {
    plan.given
        .iter()
        .map(|input| format!(", uint8_t {}", input_name(&input.name)))
        .collect()
}

/// The arguments [`given_params`] names, passed on as they are.
fn given_args(plan: &EncodePlan<'_>) -> String {
    plan.given
        .iter()
        .map(|input| format!(", {}", input_name(&input.name)))
        .collect()
}

/// Where an input the value says is read: in the encode, off the locals each
/// element was read into once, or in the input's own function a parent calls,
/// off the source it is handed.
#[derive(Clone, Copy)]
enum Read {
    Encode,
    Source,
}

/// The `uint8_t` the input `input` the value says holds (docs/adr/0065 §6
/// R4a): 1 exactly where the field gated on it alone is there (absent, for a
/// field gated on its clear state), or what the body that hands it on says,
/// read through that body's codec's own function of it, which answers an
/// optional value where the body says it in some of its values only.
/// What the arm of the variant says, the arm the source presents, is what the
/// variant's own function of it answers ([`arm_inputs`]). [`unlowered`] names
/// every other way a value says one.
fn said_input(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str, input: &str, at: Read) -> String {
    let source = |id: &str| match at {
        Read::Encode => element_local(l, id),
        Read::Source => format!("src->{}(ctx)", l.codec_field_id(id)),
    };
    match plan.roles.inputs.get(input) {
        Some(InputSource::Gate { field, when_clear }) => {
            let (set, clear) = if *when_clear { (0, 1) } else { (1, 0) };
            format!("(uint8_t)({}.present ? {set}u : {clear}u)", source(field))
        }
        // The body is always there: an input is handed on only from a body no
        // gate decides (`codec_roles::said_inputs`).
        Some(InputSource::Forward {
            embed,
            alias,
            input,
            ..
        }) => {
            let said = format!("{}_input_{}", body_prefix(alias), input_name(input));
            match at {
                Read::Encode => {
                    let held = element_local(l, embed);
                    format!("{said}({held}.table, {held}.ctx)")
                }
                // The pair the source presents the body as, its table and its
                // context, each read through the source's function of it.
                Read::Source => format!("{said}({}.table, {}.ctx)", source(embed), source(embed)),
            }
        }
        Some(InputSource::EveryArm { .. } | InputSource::SomeArms { .. }) => {
            format!("{snake}_arm_input_{}({})", input_name(input), source(BODY))
        }
        // Set exactly where the arm of the group it decides the value holds
        // sets it (docs/adr/0065 §6 R4b).
        Some(InputSource::Group) => {
            let groups = group::groups(plan, snake);
            let gate = GateVar::Input(input.to_string());
            let (g, at) = groups
                .iter()
                .find_map(|g| {
                    g.group
                        .gates
                        .iter()
                        .position(|v| *v == gate)
                        .map(|at| (g, at))
                })
                .expect("an input a group says has its group");
            format!(
                "(uint8_t)({} ? 1u : 0u)",
                g.gate_value(at, &source(&g.name))
            )
        }
        // The start of the list the codec hands it on to, which every entry
        // says (docs/adr/0065 §6 R8); a gated list says nothing while it is
        // absent, and the input's function answers an optional value then.
        Some(InputSource::List {
            list,
            input: entry_input,
            ..
        }) => {
            let held = source(list);
            let start = format!("start_{}", input_name(entry_input));
            let gated = plan
                .model
                .fields
                .iter()
                .find(|f| &f.id == list)
                .is_some_and(|f| plan.presence(f) == Presence::Gated);
            match (plan.roles.says_input_partly(input), gated) {
                (true, true) => format!(
                    "({held}.present ? sce_optional_uint8_some({held}.value.{start}) : \
                     sce_optional_uint8_none())"
                ),
                (true, false) => format!("sce_optional_uint8_some({held}.{start})"),
                (false, _) => format!("{held}.{start}"),
            }
        }
        _ => unreachable!(
            "`unlowered` prints an input a field's gate, a body, a group, an arm or a list's \
             start says only"
        ),
    }
}

/// `{snake}_input_<name>` for each input the value says, read off any source of
/// it: what a parent that holds the codec derives the flag it binds to the
/// input from, and holds what it hands the codec to (docs/adr/0065 §6 R4a). A
/// value that can say nothing of the input answers an absent value then (§6
/// R4e).
fn said_inputs(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
    let mut out = String::new();
    for said in &plan.said {
        let name = input_name(&said.input.name);
        let (answer, nothing) = if plan.roles.says_input_partly(&said.input.name) {
            (
                "sce_optional_uint8_t",
                ",\n * and an absent value where the value says nothing of it",
            )
        } else {
            ("uint8_t", "")
        };
        out.push_str(&format!(
            "\n/* The flag input `{}` a source of {snake}_t says, which its encode is\n \
             * not handed (docs/adr/0065 §6 R4a){nothing}. */\n\
             static inline {answer} {snake}_input_{name}(const {snake}_source_t *src, \
             const void *ctx) {{\n    \
             return {};\n\
             }}\n",
            said.input.name,
            said_input(plan, l, snake, &said.input.name, Read::Source)
        ));
    }
    out
}

/// The arguments the codec imported as `alias` is handed after the sink, one
/// for each input it takes and does not say, in the order it declares them
/// (docs/adr/0065 §6 R4a): the bits of a carrier as the encode writes it,
/// read by `read`, or an input of this codec's own, passed on.
fn handed_args(plan: &EncodePlan<'_>, alias: &str, read: &dyn Fn(&CodecField) -> String) -> String {
    let Some(inputs) = plan.handed.get(alias) else {
        return String::new();
    };
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
                    ", (uint8_t)(((uint64_t){} >> {bit}) & 0x{mask:X}ull)",
                    read(carrier)
                )
            }
            Some(HandedFrom::Input { name }) => format!(", {}", input_name(name)),
            // An input nothing binds, which only a chain hands its entries,
            // from the entry before ([`walk_supplies`]).
            None => format!(", {}", previous_local(&handed.input)),
        })
        .collect()
}

/// The local the walk of a chain holds, for the input `input` of its entries
/// nothing binds, whether the entry before the one it writes was of the kind
/// the input waits for (`ListWrite::previous`).
fn previous_local(input: &str) -> String {
    format!("_p_{}", input_name(input))
}

/// Whether the gate of `field` holds over the carriers the encode writes,
/// each read by `read`, and the flag inputs: its clauses joined by "or".
fn gate_test(
    plan: &EncodePlan<'_>,
    field: &CodecField,
    read: &dyn Fn(&CodecField) -> String,
) -> String {
    let gate = field
        .present_if
        .as_ref()
        .expect("a field whose gate is tested is gated");
    let bit = |value: String, mask: u64, negate: bool| {
        format!(
            "(((uint64_t){value} & 0x{mask:X}ull) {} 0)",
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
            } => bit(read(carrier), 1u64 << *at, *negate),
            Clause::Input { name, negate } => bit(input_name(name), 1, *negate),
            Clause::Member { .. } => {
                unreachable!("`unlowered` prints no gate of a chain's membership")
            }
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

/// The statements that hold the input a held codec says to what this codec
/// hands it (`check`), refused as a gate that disagrees with what it gates is
/// (docs/adr/0065 §6 R4a): the bits of a carrier as the encode writes it, read
/// by `read`, or an input of this codec's own. A value that says nothing of
/// the input disagrees with nothing (§6 R4e); a body or a list a gate leaves
/// out holds nothing that could disagree.
fn input_check(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    check: &InputCheck<'_>,
    read: &dyn Fn(&CodecField) -> String,
) -> String {
    let fail = refusal(plan, RefusalKind::PresentIfMismatch, &check.at);
    let handed = match &check.against {
        Against::Carrier { carrier, bit } => {
            format!("((((uint64_t){} >> {bit}) & 1u) != 0)", read(carrier))
        }
        Against::Input { name } => format!("({} != 0)", input_name(name)),
        Against::PreviousEntry { .. } => {
            unreachable!("`unlowered` prints no input a chain's entry says of the entry before")
        }
    };
    let said = format!(
        "{}_input_{}",
        body_prefix(&check.codec),
        input_name(&check.input)
    );
    // Whether the source `table` presents over `ctx` says the input otherwise
    // than it is handed, at the indentation `pad`.
    let otherwise = |table: &str, ctx: &str, pad: &str| {
        if check.partly {
            format!(
                "{pad}{{\n\
                 {pad}    const sce_optional_uint8_t _s = {said}({table}, {ctx});\n\
                 {pad}    if (_s.present && ((_s.value != 0) != _h)) {{\n\
                 {pad}        {fail}\n\
                 {pad}    }}\n\
                 {pad}}}\n"
            )
        } else {
            format!(
                "{pad}if ((({said}({table}, {ctx})) != 0) != _h) {{\n\
                 {pad}    {fail}\n\
                 {pad}}}\n"
            )
        }
    };
    let (test, present) = match &check.site {
        CheckSite::Embed { body } => {
            let local = element_local(l, &body.id);
            let gated = plan.presence(body) == Presence::Gated;
            let held = if gated {
                format!("{local}.value")
            } else {
                local.clone()
            };
            (
                otherwise(&format!("{held}.table"), &format!("{held}.ctx"), "        "),
                gated.then(|| format!("{local}.present")),
            )
        }
        // Every entry of the list, walked from the first.
        CheckSite::List { list } => {
            let local = element_local(l, &list.id);
            let gated = plan.presence(list) == Presence::Gated;
            let held = if gated {
                format!("{local}.value")
            } else {
                local.clone()
            };
            (
                format!(
                    "        {{\n            \
                     const void *_at = {held}.walk->begin({held}.ctx);\n            \
                     for (size_t _i = 0; _i < {held}.count && _at != NULL; ++_i) {{\n\
                     {}            \
                     _at = {held}.walk->next({held}.ctx, _at);\n            \
                     }}\n        \
                     }}\n",
                    otherwise(&format!("{held}.entry"), "_at", "                ")
                ),
                gated.then(|| format!("{local}.present")),
            )
        }
        // The arm the value holds, where it is one of the arms whose codec
        // says the input.
        CheckSite::Arm { arms: saying } => {
            let arms = Arms::of(plan);
            let held = element_local(l, BODY);
            let mut cases = String::new();
            for &i in saying {
                let arm = &arms.arms[i];
                let (table, ctx) = arm_source(&held, arm);
                cases.push_str(&arms.guarded(
                    i,
                    &format!(
                        "            case {}:\n{}                break;\n",
                        arm.kind,
                        otherwise(&table, &ctx, "                ")
                    ),
                ));
            }
            (
                format!(
                    "        switch ({held}.kind) {{\n{cases}            default:\n                \
                     break;\n        }}\n"
                ),
                None,
            )
        }
    };
    let test = match present {
        Some(there) => {
            let mut inner = String::new();
            for line in test.lines() {
                inner.push_str("    ");
                inner.push_str(line);
                inner.push('\n');
            }
            format!("        if ({there}) {{\n{inner}        }}\n")
        }
        None => test,
    };
    format!("    {{\n        const bool _h = {handed};\n{test}    }}\n")
}

/// The member of `{codec}_overrides_t` that sets `flag` of the carrier
/// `carrier`, the codec's own name for it, which a chain holding the codec
/// spells alike: the name of the flag as an element ([`number_name`]).
fn override_member(l: &LangCtx, carrier: &str, flag: &str) -> String {
    number_name(
        l,
        &ElementId::Flag {
            field: carrier.to_string(),
            flag: flag.to_string(),
        },
    )
}

/// The bits of the flag `flag` of the carrier `carrier`, below its lowest.
fn flag_mask(plan: &EncodePlan<'_>, carrier: &str, flag: &str) -> u64 {
    let width = plan
        .model
        .fields
        .iter()
        .find(|f| f.id == carrier)
        .and_then(|c| c.flags.iter().find(|d| d.name == flag))
        .map(|d| d.width.max(1))
        .expect("a flag the encode derives is a flag of a carrier of the codec");
    if width >= 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

/// The statements that or into `local` the bits the arm of each body of `tag`
/// fills (docs/adr/0081 §1, the parent's tag): the arm's tag the body's own
/// document gives, masked to the flag's bits, and for its catch-all the tag
/// it holds, masked. The arm is read off the source the value presents the
/// body as, through the function of its codec's table that presents the arm;
/// a body a gate leaves out fills nothing. An arm the body's codec compiles
/// under a build feature is a case inside the guard of its macro.
fn parent_tag(plan: &EncodePlan<'_>, l: &LangCtx, tag: &ParentTag<'_>, local: &str) -> String {
    let mut out = String::new();
    for term in &tag.terms {
        let prefix = body_prefix(&term.codec);
        let body = element_local(l, &term.body.id);
        let gated = plan.presence(term.body) == Presence::Gated;
        let open = if gated {
            format!("    if ({body}.present) {{\n")
        } else {
            "    {\n".to_string()
        };
        let held = if gated { format!("{body}.value") } else { body };
        let mut cases = String::new();
        for arm in &term.arms {
            let feature = arm.feature.as_deref().map(c_feature_macro);
            cases.push_str(&feature_guarded(
                feature.as_deref(),
                &format!(
                    "            case {}:\n                {local} |= 0x{:X}ull;\n                \
                     break;\n",
                    arm_kind(&prefix, Some(&arm.codec)),
                    arm.tag
                ),
            ));
        }
        if term.catch_all {
            cases.push_str(&format!(
                "            case {}:\n                {local} |= (uint64_t)_v.default_tag & \
                 0x{:X}ull;\n                break;\n",
                arm_kind(&prefix, None),
                term.mask
            ));
        }
        out.push_str(&format!(
            "{open}        const {prefix}_body_ref_t _v = {held}.table->{BODY}({held}.ctx);\n        \
             switch (_v.kind) {{\n\
             {cases}            default:\n                break;\n        \
             }}\n    \
             }}\n"
        ));
    }
    out
}

/// Where every body that fills the flag of `tag` is one a gate may leave out,
/// the test that one of them is there: with none there, nothing says the flag,
/// and the carrier keeps the bits the value presents (docs/adr/0065 §6 R4a), as
/// a decode that read the frame left them. `None` where a body is always
/// there.
fn parent_tag_said(plan: &EncodePlan<'_>, l: &LangCtx, tag: &ParentTag<'_>) -> Option<String> {
    tag.terms
        .iter()
        .map(|term| {
            (plan.presence(term.body) == Presence::Gated)
                .then(|| format!("{}.present", element_local(l, &term.body.id)))
        })
        .collect::<Option<Vec<String>>>()
        .map(|there| there.join(" || "))
}

/// `{snake}_overrides_t`, the one-bit flags of the codec a parent may set in
/// place of the ones a value presents, each absent where the parent sets
/// none, and `{snake}_overrides_none()`, which sets none: a chain whose
/// entries are of the codec sets the flag its entries end on this way, as an
/// entry cannot know where it stands (docs/adr/0081 §1).
fn declare_overrides(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
    let mut members = String::new();
    let mut none = String::new();
    for (carrier, def) in &plan.overridable {
        let member = override_member(l, &carrier.id, &def.name);
        members.push_str(&format!(
            "    /* The `{}` flag of `{}`, where a parent sets it. */\n    \
             sce_optional_bool_t {member};\n",
            def.name, carrier.id
        ));
        none.push_str(&format!("    none.{member} = sce_optional_bool_none();\n"));
    }
    format!(
        "/* One-bit flags of {snake}_t a parent may set in place of the ones a value\n \
         * presents, such as the flag a chain whose entries are {snake}_t ends them\n \
         * on, which the chain derives from where each entry stands. A flag absent\n \
         * here is the value's own. */\n\
         typedef struct {{\n{members}}} {snake}_overrides_t;\n\n\
         /* The overrides that set no flag. */\n\
         static inline {snake}_overrides_t {snake}_overrides_none(void) {{\n    \
         {snake}_overrides_t none;\n\
         {none}    return none;\n\
         }}\n\n"
    )
}

/// One measure of a derived number as the encode reads it.
struct MeasureText {
    /// The number, a `size_t`, read only where the measure is there.
    size: String,
    /// Where the measure may be absent, the test that it is there.
    present: Option<String>,
    /// Whether the plan says the measure may be absent (`Measure::gated`).
    gated: bool,
}

/// A flag of a carrier the encode derives, folded into the carrier.
struct Fold {
    /// The flag.
    flag: String,
    /// The element it is, as the totality analysis names it (`carrier.flag`).
    named: String,
    /// What its bits hold, as a `uint64_t`.
    bits: String,
    /// Where it holds a number, the number as a `size_t`, which a refusal of
    /// a number past its bits reads.
    count: Option<String>,
    /// Where it holds a number of content a gate may leave out, the test that
    /// the content is there; the carrier keeps the bits the value presents
    /// where it is not.
    present: Option<String>,
}

/// The statement that returns the refusal of `kind` at `at`, which the
/// totality analysis names (`Totality::require`, docs/adr/0065 §2), as the C
/// runtime's status (`EncodeFailure::symbol`).
fn refusal(plan: &EncodePlan<'_>, kind: RefusalKind, at: &str) -> String {
    plan.totality.require(kind, at);
    let status = EncodeFailure::ALL
        .into_iter()
        .find(|failure| failure.refusal_kind() == kind)
        .and_then(|failure| failure.symbol(crate::generator::Language::C11))
        .expect("a positional encode refuses with a status the C runtime names");
    format!("return {status};")
}

/// `refused`, a statement at the indentation of an encode's own, where `test`
/// holds.
fn refuse_if(test: &str, refused: &str) -> String {
    format!("    if ({test}) {{\n        {refused}\n    }}\n")
}

/// `measure` as the encode reads it, off the local the source's element was
/// read into: the bytes a view holds, less or more its `sce:length-arith`, a
/// length less a number the bytes may be shorter than refused; the entries a
/// list's maker counted; the input a body says, read through its codec's own
/// function of it; or the bytes a
/// body encodes to, counted by its codec's encode over the source the value
/// presents it as into a counting sink, which stores nothing (docs/adr/0058
/// §3) and passes on what the body refuses, handed what `handed` answers for
/// the body's codec, which the encode answers once the carriers are written.
/// A body is counted once into
/// `_k_<body>`, which `counted` keeps, and only where it is there. The input
/// the arm the value holds says is read into `_s_<element>`, `element` the
/// number the measure is of. The statements that count, read or refuse go
/// into `out`.
fn measure_text(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    element: &ElementId,
    measure: &Measure<'_>,
    handed: Option<&dyn Fn(&str) -> String>,
    counted: &mut BTreeSet<String>,
    out: &mut String,
) -> MeasureText {
    match &measure.of {
        Measured::Bytes { field, offset } => {
            let local = element_local(l, &field.id);
            let (len, present) = if measure.gated {
                (
                    format!("{local}.value.len"),
                    Some(format!("{local}.present")),
                )
            } else {
                (format!("{local}.len"), None)
            };
            if let Offset::Less { by, held: false } = offset {
                let short = format!("{len} < {by}u");
                let test = match &present {
                    Some(there) => format!("{there} && {short}"),
                    None => short,
                };
                out.push_str(&refuse_if(
                    &test,
                    &refusal(plan, RefusalKind::LengthOutOfRange, &field.id),
                ));
            }
            let size = match offset {
                Offset::None => len,
                Offset::Less { by, .. } => format!("{len} - {by}u"),
                Offset::More { by } => format!("{len} + {by}u"),
            };
            MeasureText {
                size,
                present,
                gated: measure.gated,
            }
        }
        Measured::Encoded { body, codec, .. } => {
            let local = element_local(l, &body.id);
            let sink = format!("_k_{}", l.codec_field_id(&body.id));
            let held = if measure.gated {
                format!("{local}.value")
            } else {
                local.clone()
            };
            let present = measure.gated.then(|| format!("{local}.present"));
            if counted.insert(body.id.clone()) {
                let handed =
                    handed.expect("an encoded length is counted once the carriers are written");
                let count = format!(
                    "    SCE_FORGE_TRY_WRITE({}_encode_source({held}.table, {held}.ctx, &{sink}{}));\n",
                    body_prefix(codec),
                    handed(codec)
                );
                out.push_str(&format!(
                    "    sce_forge_writer_t {sink} = sce_forge_writer_init_counting();\n"
                ));
                out.push_str(&match &present {
                    Some(there) => guarded(there, &count),
                    None => count,
                });
            }
            MeasureText {
                size: format!("sce_forge_writer_position(&{sink})"),
                present,
                gated: measure.gated,
            }
        }
        // The entries the list's maker counted, within its bound.
        Measured::Entries { list } => {
            let local = element_local(l, &list.id);
            if measure.gated {
                MeasureText {
                    size: format!("{local}.value.count"),
                    present: Some(format!("{local}.present")),
                    gated: true,
                }
            } else {
                MeasureText {
                    size: format!("{local}.count"),
                    present: None,
                    gated: false,
                }
            }
        }
        // The input the body says, read through its codec's own function of
        // it off the source the value presents the body as (docs/adr/0065 §6
        // R4a): the roles derive a flag only from a body that says it in every
        // value, so the function answers a number. Where a gate leaves the
        // body out it says nothing: a field beside it holds zero and a flag
        // keeps the bits the value presents.
        Measured::Says { body, codec, input } => {
            let local = element_local(l, &body.id);
            let held = if measure.gated {
                format!("{local}.value")
            } else {
                local.clone()
            };
            MeasureText {
                size: format!(
                    "(size_t){}_input_{}({held}.table, {held}.ctx)",
                    body_prefix(codec),
                    input_name(input)
                ),
                present: measure.gated.then(|| format!("{local}.present")),
                gated: measure.gated,
            }
        }
        // The input the arm the value holds says, read through its codec's
        // own function of it off the source of the arm's body (docs/adr/0065
        // §6 R4a): the roles derive a flag only from arms that say it in every
        // value, so the function answers a number. Where the value holds an
        // arm that does not say it, a field beside it holds zero and a flag
        // keeps the bits the value presents.
        Measured::ArmSays { arms: saying } => {
            let arms = Arms::of(plan);
            let held = element_local(l, BODY);
            let local = format!("_s_{}", number_name(l, element));
            let there = measure.gated.then(|| format!("{local}_there"));
            out.push_str(&format!("    size_t {local} = 0;\n"));
            if let Some(there) = &there {
                out.push_str(&format!("    bool {there} = false;\n"));
            }
            let places: Vec<usize> = saying.iter().map(|said| said.arm).collect();
            out.push_str(&arms.switch(&format!("{held}.kind"), &places, |arm| {
                let input = saying
                    .iter()
                    .find(|said| arms.arms[said.arm].member == arm.member)
                    .map(|said| input_name(&said.input))
                    .expect("an arm the switch names says the input");
                let (table, ctx) = arm_source(&held, arm);
                let mark = there
                    .as_ref()
                    .map(|there| format!("            {there} = true;\n"))
                    .unwrap_or_default();
                format!(
                    "            {local} = (size_t){}_input_{input}({table}, {ctx});\n{mark}",
                    arm.prefix
                )
            }));
            MeasureText {
                size: local,
                present: there,
                gated: measure.gated,
            }
        }
        // The start the list's law gives every entry, which the list its
        // maker made holds (docs/adr/0065 §6 R8). Where a gate leaves the list
        // out it says nothing: a field beside it holds zero and a flag keeps
        // the bits the value presents.
        Measured::Start { list, input } => {
            let local = element_local(l, &list.id);
            let name = input_name(input);
            if measure.gated {
                MeasureText {
                    size: format!("(size_t){local}.value.start_{name}"),
                    present: Some(format!("{local}.present")),
                    gated: true,
                }
            } else {
                MeasureText {
                    size: format!("(size_t){local}.start_{name}"),
                    present: None,
                    gated: false,
                }
            }
        }
    }
}

/// The name of the locals the encode makes the derived number of `element`
/// in: its field's, or its carrier's and its flag's.
fn number_name(l: &LangCtx, element: &ElementId) -> String {
    match element {
        ElementId::Field(id) => l.codec_field_id(id),
        ElementId::Flag { field, flag } => format!(
            "{}_{}",
            l.codec_field_id(field),
            filters::to_snake_case(flag.clone())
        ),
        ElementId::Variant => unreachable!("a number is derived into a field or a flag"),
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

/// The statements, written into `out`, that make the derived number of
/// `element` from `measures`, as the Rust encode's `combine` makes it
/// (docs/adr/0081 §1): the first measure always there fixes it, and every
/// other one there must say the same, or the value is refused as
/// `INCONSISTENT_SOURCE`; where every measure may be absent, it is the first
/// one there. A field holds it as `_d_<field>`, refused as `LENGTH_OUT_OF_RANGE`
/// where it may not fit; where nothing it measures is there, the field holds
/// zero, or what the value states, which must then be what the content says
/// where there is content. A flag's number is answered as the fold of it into
/// its carrier, by the carrier's id.
fn derive_number<'e>(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    element: &'e ElementId,
    into: &NumberInto<'_>,
    measures: &[MeasureText],
    out: &mut String,
) -> Option<(&'e str, Fold)> {
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
            let always: Vec<&MeasureText> = measures.iter().filter(|m| !m.gated).collect();
            let gated: Vec<&MeasureText> = measures.iter().filter(|m| m.gated).collect();
            if let Some((anchor, others)) = always.split_first() {
                for other in others {
                    out.push_str(&refuse_if(
                        &format!("(size_t)({}) != (size_t)({})", other.size, anchor.size),
                        &disagree(),
                    ));
                }
                for g in &gated {
                    out.push_str(&refuse_if(
                        &format!(
                            "{} && (size_t)({}) != (size_t)({})",
                            there(g),
                            g.size,
                            anchor.size
                        ),
                        &disagree(),
                    ));
                }
                (anchor.size.clone(), None)
            } else {
                let first = format!("_m_{}", number_name(l, element));
                let first_there = format!("{first}_there");
                out.push_str(&format!(
                    "    size_t {first} = 0;\n    bool {first_there} = false;\n"
                ));
                for (i, g) in gated.iter().enumerate() {
                    let keyword = if i == 0 { "if" } else { "} else if" };
                    out.push_str(&format!(
                        "    {keyword} ({}) {{\n        {first} = (size_t)({});\n        \
                         {first_there} = true;\n",
                        there(g),
                        g.size
                    ));
                }
                out.push_str("    }\n");
                for g in gated.iter().skip(1) {
                    out.push_str(&refuse_if(
                        &format!("{} && (size_t)({}) != {first}", there(g), g.size),
                        &disagree(),
                    ));
                }
                (first, Some(first_there))
            }
        }
    };
    match (element, into) {
        (
            ElementId::Field(_),
            NumberInto::Field {
                field,
                narrow,
                absent,
            },
        ) => {
            let ty = l.type_name(&field.sce_type);
            let local = derived_local(&field.id);
            // A number past what the field holds is no frame (docs/adr/0065
            // §5), asked of the runtime as a bound is.
            if *narrow == Narrow::Checked {
                let most = largest_of(&field.sce_type)
                    .expect("a length that may not fit its field is held in an integer field");
                let past = format!("!sce_forge_length_within_bound((size_t)({size}), {most})");
                let test = match &present {
                    Some(there) => format!("{there} && {past}"),
                    None => past,
                };
                out.push_str(&refuse_if(
                    &test,
                    &refusal(plan, RefusalKind::LengthOutOfRange, &named),
                ));
            }
            // Where nothing it measures is there: zero, as the field is then
            // gated beside its content; what the value states; or what the
            // arm of the group the value holds holds on the side where the
            // content is not, bound to the field's local as a member of a
            // group is, zero where the arm holds none (docs/adr/0065 §6 R4b).
            // The two sides of a group are apart, so the number is never
            // said twice there.
            let stated = element_local(l, &field.id);
            let otherwise = match absent {
                Absent::Never | Absent::Zero => format!("({ty})0"),
                Absent::Stated | Absent::Group => {
                    format!("({stated}.present ? {stated}.value : ({ty})0)")
                }
            };
            let value = match &present {
                None => format!("({ty})({size})"),
                Some(there) => format!("{there} ? ({ty})({size}) : {otherwise}"),
            };
            out.push_str(&format!("    const {ty} {local} = {value};\n"));
            // A value that states the number beside content says it twice,
            // and must say what the content does.
            if *absent == Absent::Stated {
                let there = present
                    .as_deref()
                    .expect("a number the value states is free: every measure may be absent");
                out.push_str(&refuse_if(
                    &format!("{there} && {stated}.present && {stated}.value != {local}"),
                    &disagree(),
                ));
            }
            None
        }
        (ElementId::Flag { field, flag }, NumberInto::Flag) => Some((
            field.as_str(),
            Fold {
                flag: flag.clone(),
                named,
                bits: format!("(uint64_t)({size})"),
                count: Some(size),
                present,
            },
        )),
        _ => unreachable!(
            "a number derived into a field goes into the field, and into a flag into the flag"
        ),
    }
}

/// `body`, statements at the indentation of an encode's own, written where
/// `test` holds.
fn guarded(test: &str, body: &str) -> String {
    let mut out = format!("    if ({test}) {{\n");
    for line in body.lines() {
        if !line.is_empty() {
            out.push_str("    ");
            out.push_str(line);
        }
        out.push('\n');
    }
    out.push_str("    }\n");
    out
}

/// [`C11Source::encode_body`]: the value checked by [`check_value`] before the
/// first byte, then encoded over the value's table, handed the flag inputs the
/// value does not say, which the value's encode takes as its own parameters.
fn encode_body(plan: &EncodePlan<'_>, snake: &str) -> String {
    format!(
        "    /* The value within its bounds before the first byte (docs/adr/0081 §2). */\n    \
         SCE_FORGE_TRY_WRITE({snake}_check_value(self));\n    \
         return {snake}_encode_source({snake}_value_source(), self, w{});\n",
        given_args(plan)
    )
}

/// `{snake}_check_value`, which answers whether a value of the codec's struct
/// is a value of the codec. The value holds each byte string and text in an
/// array beside its length, as C holds a value with no allocator, and C cannot
/// keep that length within the bound of the element's view; a value whose
/// length is outside it, or whose text no UTF-8 holds, is no value of the
/// codec, and is refused before the first byte, as a decode would refuse its
/// frame. A list's entries are held in an array beside their count in the same
/// way, held to the most the list holds. A body or an entry it holds is
/// checked by its own codec's check, as the value's table presents the body's
/// and the entry's arrays through that codec's views.
fn check_value(plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
    let mut out = String::new();
    for element in &plan.elements {
        // The array of a list within the most its list holds, and each entry
        // it holds a value of its codec, by that codec's own check; where a
        // gate decides the list, only where it is present.
        if let Value::List { codec, law, .. } = &element.value {
            let list = BoundedList::of(l, snake, element.field, &element.value);
            let name = l.codec_field_id(&element.field.id);
            let held = u64::from(c11_held_entries(element.field, &plan.model.fields));
            let most = list.bound.most.map_or(held, |most| most.min(held));
            let (data, len) = if element.presence == Presence::Gated {
                (
                    format!("self->{name}.value.data"),
                    format!("self->{name}.value.len"),
                )
            } else {
                (format!("self->{name}"), format!("self->{name}_len"))
            };
            let mut checks = format!(
                "    if (!sce_forge_length_within_bound({len}, {most})) {{\n        \
                 return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;\n    }}\n"
            );
            if list.bound.least > 0 {
                checks.push_str(&format!(
                    "    if ({len} < {}u) {{\n        \
                     return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }}\n",
                    list.bound.least
                ));
            }
            checks.push_str(&format!(
                "    for (size_t _i = 0; _i < {len}; ++_i) {{\n        \
                 SCE_FORGE_TRY_WRITE({}_check_value(&{data}[_i]));\n    }}\n",
                body_prefix(codec)
            ));
            // The entries held to the list's law from the start the value
            // holds beside it, by the list's own maker over the value's walk
            // of them (docs/adr/0065 §6 R8).
            if !law.is_none() {
                let made = if element.presence == Presence::Gated {
                    format!("{snake}_value_{name}(self).value")
                } else {
                    format!("{snake}_value_{name}(self)")
                };
                let starts: String = law_starts(&element.value)
                    .iter()
                    .map(|input| format!(", _l.start_{input}"))
                    .collect();
                let (ty, base) = (list.ty(), &list.base);
                checks.push_str(&format!(
                    "    {{\n        \
                     const {ty} _l = {made};\n        \
                     {ty} _made;\n        \
                     SCE_FORGE_TRY_WRITE({base}_make(&_made, _l.walk, _l.ctx, _l.entry{starts}));\n    \
                     }}\n"
                ));
            }
            out.push_str(&if element.presence == Presence::Gated {
                guarded(&format!("self->{name}.present"), &checks)
            } else {
                checks
            });
            continue;
        }
        if let Value::Body { codec } = &element.value {
            let name = l.codec_field_id(&element.field.id);
            let check = format!("{}_check_value", body_prefix(codec));
            out.push_str(&if element.presence == Presence::Gated {
                format!(
                    "    if (self->{name}.present) {{\n        \
                     SCE_FORGE_TRY_WRITE({check}(&self->{name}.value));\n    }}\n"
                )
            } else {
                format!("    SCE_FORGE_TRY_WRITE({check}(&self->{name}));\n")
            });
            continue;
        }
        if !matches!(element.value, Value::Bytes(_) | Value::Text(_)) {
            continue;
        }
        let view = BoundedView::of(l, snake, element.field, &element.value);
        let name = l.codec_field_id(&element.field.id);
        // The array's own bound, where it is below the element's.
        let held = u64::from(c11_held_bytes(element.field, &plan.model.fields));
        let most = view.bound.most.map_or(held, |most| most.min(held));
        // One a gate decides is held in the optional value of its bytes, and
        // held to the bound where it is present.
        let (data, len, present) = if element.presence == Presence::Gated {
            (
                format!("self->{name}.value.data"),
                format!("self->{name}.value.len"),
                Some(format!("self->{name}.present")),
            )
        } else {
            (
                format!("(const uint8_t *)self->{name}"),
                format!("self->{name}_len"),
                None,
            )
        };
        let mut refusals = Vec::new();
        if let Some(outside) = length_outside(&len, view.bound.least, Some(most)) {
            refusals.push((outside, "SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE"));
        }
        if view.text {
            refusals.push((
                format!("!sce_forge_is_valid_utf8({data}, {len})"),
                "SCE_FORGE_CODEC_INVALID_UTF8",
            ));
        }
        for (refused, status) in refusals {
            let refused = match &present {
                Some(present) => format!("{present} && ({refused})"),
                None => refused,
            };
            out.push_str(&format!(
                "    if ({refused}) {{\n        return {status};\n    }}\n"
            ));
        }
    }
    // The members of the arm of each group the value holds, each held to what
    // its type holds as an element of the type is.
    for g in group::groups(plan, snake) {
        out.push_str(&g.check(plan, l, snake));
    }
    // The body of the arm the value holds, a value of the arm's codec by that
    // codec's own check.
    if plan.model.variant.is_some() {
        let arms = Arms::of(plan);
        out.push_str(
            &arms.switch(&format!("self->{BODY}.kind"), &arms.all(), |arm| {
                format!(
                    "            SCE_FORGE_TRY_WRITE({}_check_value(&self->{BODY}.arm.{}));\n",
                    arm.prefix, arm.member
                )
            }),
        );
    }
    if out.is_empty() {
        out.push_str("    (void)self;\n");
    }
    format!(
        "/* Whether `self` is a value of {snake}_t: SCE_FORGE_CODEC_OK, or the\n \
         * refusal of an array past the bound of its element's view or list or of a\n \
         * text no UTF-8 holds, in it or in a body or an entry it holds\n \
         * (docs/adr/0081 §2). */\n\
         static inline sce_forge_codec_status_t {snake}_check_value(const {snake}_t *self) {{\n\
         {out}    return SCE_FORGE_CODEC_OK;\n\
         }}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The kind and the union member of an arm are the arm's name every
    /// backend declares it by, in C's cases, and the catch-all's member keeps
    /// apart from an arm named `Default`.
    ///
    /// Turns red where `arm_kind` or `arm_member` spells the alias itself and
    /// not `variant_arm_name` of it, which differ for `x_2` (`X2`).
    #[test]
    fn an_arm_is_named_in_c_by_the_name_every_backend_gives_it() {
        assert_eq!(variant_arm_name(Some("x_2")), "X2");
        assert_eq!(arm_kind("codec_a", Some("x_2")), "CODEC_A_BODY_KIND_X2");
        assert_eq!(arm_member(Some("x_2")), "x2");
        assert_eq!(
            arm_kind("codec_a", Some("codec_variant_session_open")),
            "CODEC_A_BODY_KIND_CODEC_VARIANT_SESSION_OPEN"
        );
        assert_eq!(arm_kind("CodecA", None), "CODEC_A_BODY_KIND_DEFAULT");
        assert_eq!(arm_member(None), "default_body");
        assert_eq!(arm_member(Some("default")), "default");
    }
}
