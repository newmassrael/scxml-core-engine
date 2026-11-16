// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Kotlin printer of a codec's encode plan (docs/adr/0081 §2, step 4).
//!
//! A Kotlin value presents its elements as the properties of an interface,
//! `{Codec}Source`, which the codec's data class implements and any other
//! class may, and its encode is a function over any source,
//! `{Codec}.encodeSource` of the data class's companion object beside its
//! `decode`: it reads each element the source presents once and derives every
//! other element of the frame from them, as the plan says, never from the
//! value (docs/adr/0058 §1). A byte string or a text is presented as a value
//! class that names its bound, the abstract type of §2 for Kotlin: a class
//! whose constructor is private and whose one factory, `of`, checks the bound,
//! over bytes whose count or a text that cannot change after, so a value of it
//! holds its bound for as long as it lives and the encode over it refuses
//! nothing the bound decides. An element a gate decides is `null` where it is
//! absent, and the flag that says so is derived from it. What the encode does
//! refuse, it answers before the first byte, as the runtime's `CodecError`
//! object of the refusal's name.
//!
//! The printer reads the model only through the plan and lowers its forms one
//! landing at a time (§6): [`unlowered`] names the first form a codec's plan
//! uses that it does not print, and such a codec keeps the template's encode
//! whole, so no codec has two encodes.

use super::*;
use crate::forge::codec_roles::{ElementId, InputSource};
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_plan::{
    Absent, Against, Bound, CheckSite, Clause, CodecPlan, Derive, Element, EncodePlan, Guard,
    HandedFrom, Measure, Measured, Narrow, Number, NumberInto, Offset, Presence, PresenceFlag,
    Value, Write,
};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

pub(super) mod source_case;

/// The first form of `plan` the Kotlin printer does not print, as the clause
/// that names it, or `None` where it prints the codec whole.
pub(super) fn unlowered(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Option<String> {
    unlowered_forms(plan, imports).into_iter().next()
}

/// Every form of `plan` the Kotlin printer does not print, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole.
///
/// It prints a positional codec whose elements are integers, flag sets, enums,
/// byte strings, texts, and bodies and lists of codecs it prints, each always
/// there or decided by a gate on a flag of the codec's own, and whose variant's
/// arms are codecs it prints; whose every derived element is either a length,
/// a count or an encoded length of the elements it measures, held in a field
/// of its own and gated beside its content where its content is, a flag set
/// exactly where the one field it gates is there, the tag of the arm the value
/// holds, or the bits of a carrier the arm of an embedded body's variant
/// fills; and whose encode refuses
/// only what it answers itself, before the first byte: a gate it cannot
/// derive, measures that disagree, a number past its field, or what a codec
/// it holds refuses. A CBOR map and a content-line component keep their own
/// templates' encodes.
/// `imports` are the codec's own, which say whether a codec it holds is printed
/// ([`ImportContext::kotlin_printed`]).
pub(super) fn unlowered_forms(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    match plan {
        CodecPlan::Positional(plan) => positional_forms(plan, imports),
        CodecPlan::Cbor(_) => vec!["a CBOR map".to_string()],
        CodecPlan::ContentLine(_) => vec!["a content-line component".to_string()],
    }
}

/// The import of the codec a body of `plan` holds as `alias`, where the
/// Kotlin backend prints it from its plan.
fn printed_import<'i>(imports: &'i [ImportContext], alias: &str) -> Option<&'i ImportContext> {
    imports
        .iter()
        .find(|import| import.alias == alias && import.kotlin_printed)
}

/// [`unlowered_forms`] of a positional codec.
fn positional_forms(plan: &EncodePlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    let m = plan.model;
    let mut forms = Vec::new();
    // A variant is presented as the arm the value holds, each arm a source of
    // its codec, and written by that codec's own `encodeSource`, which a codec
    // the Kotlin backend prints has. An arm a build feature decides
    // (docs/adr/0072) is an arm of every Kotlin build, as the template keeps it.
    for arm in &plan.arms {
        if printed_import(imports, arm.codec).is_none() {
            forms.push(format!(
                "a variant: the arm `{}` is a codec the Kotlin backend does not print",
                arm.codec
            ));
        }
        forms.extend(handed_form(plan, "<variant>", arm.codec));
    }
    // An input the value says is read off it, by a function of the companion a
    // parent calls; one it does not say is a parameter of the encode. The
    // printer reads one a gated field or a body always there says in every
    // value.
    for said in &plan.said {
        match &plan.roles.inputs[&said.input.name] {
            InputSource::Gate { .. } | InputSource::Forward { partly: false, .. } => {}
            _ => forms.push(format!(
                "input '{}': a flag input the value says through a group, a variant, a list \
                 or a body that says it in some values only",
                said.input.name
            )),
        }
    }
    // An input a held codec says, held to what the codec is handed: a body's
    // or each entry's, against a carrier's bit or an input of the codec's own.
    for check in &plan.input_checks {
        let site = matches!(check.site, CheckSite::Embed { .. } | CheckSite::List { .. });
        let against = matches!(
            check.against,
            Against::Carrier { .. } | Against::Input { .. }
        );
        if check.partly || !site || !against {
            forms.push(format!(
                "element '{}': an input a held codec says, held to what an arm, the entry \
                 before or a codec that says it in some values only is handed",
                check.at
            ));
        }
    }
    for element in &plan.elements {
        let id = &element.field.id;
        match &element.value {
            Value::Scalar(t) if t.int_bit_width().is_some() => {}
            Value::Scalar(SceType::Enum(_)) => {}
            Value::Scalar(t) => forms.push(format!("field '{id}': a `{}` field", t.as_attr())),
            Value::Bytes(_) | Value::Text(_) => {}
            // A body is written by its codec's own `encodeSource`, which a
            // codec the Kotlin backend prints has, handed nothing.
            Value::Body { codec } => {
                if printed_import(imports, codec).is_none() {
                    forms.push(format!(
                        "field '{id}': a body of `{codec}`, which the Kotlin backend does not print"
                    ));
                }
                forms.extend(handed_form(plan, id, codec));
            }
            // A list is walked, each entry written by its codec's own
            // `encodeSource`, which a codec the Kotlin backend prints has.
            Value::List { codec, law, bound } => {
                if bound.least > 0 {
                    forms.push(format!("field '{id}': a list of a least past zero"));
                }
                if printed_import(imports, codec).is_none() {
                    forms.push(format!(
                        "field '{id}': a list of `{codec}`, which the Kotlin backend does not print"
                    ));
                }
                forms.extend(handed_form(plan, id, codec));
                if !law.is_none() {
                    forms.push(format!("field '{id}': a list held to a law"));
                }
            }
        }
        // A number the value states while its content is absent is `null`
        // where any of its content is there.
        match element.presence {
            Presence::Always | Presence::Stated => {}
            Presence::Gated => forms.extend(gate_form(plan, element.field)),
        }
    }
    if !plan.groups().is_empty() {
        forms.push("a group of fields a gate decides together".to_string());
    }
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            // A number into a field, or into a flag of a carrier.
            (_, Derive::Number(number)) => {
                forms.extend(number_form(&derived.element, number, imports));
            }
            (ElementId::Flag { .. }, Derive::Presence(presence)) => {
                forms.extend(presence_form(&derived.element, presence));
            }
            // The tag of the arm the value holds, into a field of an unsigned
            // number or a flag of a carrier; and a carrier's bits the arm of an
            // embedded body's variant fills, whose body the elements above
            // hold to a printed codec.
            (ElementId::Field(id), Derive::VariantTag) => {
                let unsigned = m.fields.iter().find(|f| &f.id == id).is_some_and(|f| {
                    f.sce_type.int_bit_width().is_some() && !f.sce_type.is_signed()
                });
                if !unsigned {
                    forms.push(format!(
                        "element '{id}': a variant's tag held in a field of no unsigned number"
                    ));
                }
            }
            (ElementId::Flag { .. }, Derive::VariantTag | Derive::ParentTag(_)) => {}
            (element, _) => forms.push(format!(
                "element '{element}': an element the encode derives other than a length \
                 or a presence flag"
            )),
        }
    }
    // A gate the encode cannot derive the presence from is a check the value
    // is held to before the first byte, tested on the carriers as the encode
    // writes them.
    for field in &plan.gate_checks {
        forms.extend(gate_form(plan, field));
    }
    let derived = derived_fields(plan);
    for write in &plan.program {
        match write {
            Write::Whole(field) | Write::Tail { field, .. } => {
                forms.extend(unread_form(plan, &derived, field));
            }
            Write::Byte { pieces, .. } => {
                for (field, _) in pieces {
                    forms.extend(unread_form(plan, &derived, field));
                }
            }
            Write::Streamed { field, guard, list } => {
                forms.extend(unread_form(plan, &derived, field));
                // A walk of each entry in turn; a chain that sets its entries'
                // flag, hands an entry what the one before it was, holds a
                // bare entry's place or is held to a law is not printed yet.
                if list.as_ref().is_some_and(|walk| {
                    !walk.previous.is_empty()
                        || walk.slots.is_some()
                        || walk.ends.is_some()
                        || walk.lawful
                }) {
                    forms.push(format!(
                        "field '{}': a chain whose walk sets or hands its entries a flag",
                        field.id
                    ));
                }
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

/// What of the derived number of `element` the printer does not print: it
/// prints a length, a count or an encoded length of the elements it measures,
/// each of which that is there says the same number or the value is refused,
/// into a field that holds nothing else, refused where it may not fit; where
/// nothing it measures is there the field holds zero, gated beside its
/// content.
fn number_form(element: &ElementId, number: &Number<'_>, imports: &[ImportContext]) -> Vec<String> {
    let mut forms = Vec::new();
    for measure in &number.measures {
        match &measure.of {
            Measured::Bytes { .. } | Measured::Entries { .. } | Measured::Encoded { .. } => {}
            // What a body says in every value, read by its codec's function.
            Measured::Says { codec, input, .. } if !says_partly(imports, codec, input) => {}
            Measured::Says { .. } | Measured::Start { .. } | Measured::ArmSays { .. } => {
                forms.push(format!(
                    "element '{element}': a flag a list, an arm or a body that says it in \
                     some values only says"
                ));
            }
        }
    }
    match &number.into {
        NumberInto::Field { absent, .. } => {
            if *absent == Absent::Group {
                forms.push(format!(
                    "element '{element}': a length a group holds on the side without its content"
                ));
            }
        }
        // Folded into its carrier, as a presence flag is.
        NumberInto::Flag => {}
    }
    forms
}

/// Whether the codec imported as `alias` says its input `input` in some of its
/// values only (docs/adr/0065 §6 R4e), so that reading it answers no value at
/// times.
fn says_partly(imports: &[ImportContext], alias: &str, input: &str) -> bool {
    imports.iter().any(|import| {
        import.alias == alias && import.codec_partly_says_inputs.iter().any(|i| i == input)
    })
}

/// The function of the companion of a printed codec that reads its input
/// `input` off any source of it (docs/adr/0065 §6 R4a).
fn said_function(input: &str) -> String {
    format!("input{}", filters::to_pascal_case(input.to_string()))
}

/// What of the presence flag `element` the printer does not print: it prints
/// a flag the presence of one field decides, or of fields one group holds.
fn presence_form(element: &ElementId, presence: &PresenceFlag<'_>) -> Option<String> {
    (presence.of.len() != 1 && !presence.together)
        .then(|| format!("element '{element}': a flag the presence of several fields decides"))
}

/// What of the gate of `field`, a field a gate decides, the printer does not
/// print: it tests a flag of the codec's own carriers or an input the encode
/// is handed.
fn gate_form(plan: &EncodePlan<'_>, field: &CodecField) -> Option<String> {
    let gate = field.present_if.as_ref()?;
    plan.clauses(gate)
        .iter()
        .any(|clause| matches!(clause, Clause::Member { .. }))
        .then(|| format!("field '{}': a gate on a chain's entries", field.id))
}

/// What of the inputs the codec imported as `codec`, held by the element `id`,
/// is handed that the printer does not print: it hands a carrier's bits or an
/// input of the codec's own, and not what the entry before was, which a chain
/// that hands it says.
fn handed_form(plan: &EncodePlan<'_>, id: &str, codec: &str) -> Option<String> {
    plan.handed
        .get(codec)
        .is_some_and(|handed| handed.iter().any(|h| h.from.is_none()))
        .then(|| format!("field '{id}': a codec handed what the entry before it was"))
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

/// Whether the printer cannot read `field` as a write reads it: the value
/// presents it alone, or the encode derives it whole.
fn unread_form(
    plan: &EncodePlan<'_>,
    derived: &BTreeSet<String>,
    field: &CodecField,
) -> Option<String> {
    (element_of(plan, &field.id).is_none() && !derived.contains(&field.id)).then(|| {
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

/// Whether the Kotlin value of `plan`, a plan [`unlowered`] answers `None` for,
/// holds no field `id`: the encode derives it (docs/adr/0081 §3) and the value
/// does not state it while its content is absent.
pub(super) fn derived_field(plan: &EncodePlan<'_>, id: &str) -> bool {
    derived_fields(plan).contains(id) && element_of(plan, id).is_none()
}

/// The fields the number the value states as `id` measures, each gated: the
/// value states the number only while none of them is there.
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

/// The class the Kotlin value of the codec whose data class is `struct_name`
/// holds its element `id` of type `ty` in, where the element is a byte string
/// or a text, which the value holds within its bound (docs/adr/0065 §6 R1).
/// The one place the name is made: the printer declares the class by it and
/// the conformance harness makes a value of it by it.
pub(crate) fn refinement_class(struct_name: &str, id: &str, ty: &SceType) -> String {
    let pascal = filters::to_pascal_case(id.to_string());
    match ty {
        SceType::String => format!("{struct_name}{pascal}Text"),
        _ => format!("{struct_name}{pascal}Bytes"),
    }
}

/// How the Kotlin value holds an element that is no Kotlin type of its own, or
/// a number it states only while its content is absent: the type its property
/// is declared as, the value it holds by default, and what its decode makes it
/// of from the locals it read.
pub(super) struct Held {
    /// The type of the property.
    pub(super) ty: String,
    /// The value it holds by default.
    pub(super) default: String,
    /// What the decode hands the constructor for it.
    pub(super) made: String,
}

/// The Kotlin text the printer gives a codec's file.
pub(super) struct KotlinSource {
    /// Before the value's class: the class of each byte string's and text's
    /// bound and the interface a source of the codec implements.
    pub(super) before: String,
    /// The encode over any source, `encodeSource`, a function of the value's
    /// companion object, at its indentation there.
    pub(super) encode: String,
    /// For each element the value holds in a class of the printer's, and each
    /// number it states while its content is absent, by field id, how it holds
    /// it.
    pub(super) held: BTreeMap<String, Held>,
    /// Whether the file declares a value class, for which it imports
    /// `JvmInline`.
    pub(super) inline: bool,
    /// The class the catch-all arm of the codec's variant holds its tag in
    /// (docs/adr/0065 §6 R3), where the variant has a catch-all: the template
    /// declares the arm with it and its decode makes the tag it read by its
    /// factory.
    pub(super) unknown_tag: Option<String>,
    /// The statements the value's own `encode`, which a caller hands every
    /// flag input, runs before it writes: each input the value says held to
    /// what it is handed, as a frame of the value says it (docs/adr/0065 §6
    /// R4a), at the indentation of a method's body; empty where the value says
    /// none.
    pub(super) said_held: String,
}

/// The data class of the codec a printed import names, which the wildcard
/// import of its package (`ImportContext::include_stmt`) brings into scope with
/// its source interface, `{Type}Source`.
fn body_type(import: &ImportContext) -> &str {
    &import.type_name
}

/// The Kotlin text of the codec `plan`, whose value's data class is
/// `struct_name`; the plan is one [`unlowered`] answers `None` for.
pub(super) fn render(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> KotlinSource {
    let mut before = String::new();
    let mut held = BTreeMap::new();
    // Whether the file declares a value class of a bound.
    let mut inline = false;
    // The type the source presents each element as.
    let mut presented: BTreeMap<String, String> = BTreeMap::new();
    for element in &plan.elements {
        let id = &element.field.id;
        let gated = element.presence == Presence::Gated;
        let optional = |ty: String| {
            if gated {
                l.optional_of(&ty).expect("Kotlin wraps the present type")
            } else {
                ty
            }
        };
        // A body is any source of its codec, the interface its file declares;
        // the value holds its codec's data class, which implements it, so the
        // value's property overrides the interface's with a type of its own.
        if let Value::Body { codec } = &element.value {
            let import = printed_import(imports, codec)
                .expect("`unlowered` prints a body of a printed codec only");
            presented.insert(id.clone(), optional(format!("{}Source", body_type(import))));
            continue;
        }
        let class = match &element.value {
            Value::Bytes(bound) | Value::Text(bound) => {
                let class = refinement_class(struct_name, id, &element.field.sce_type);
                let text = matches!(element.value, Value::Text(_));
                before.push_str(&bounded_class(struct_name, id, &class, bound, text));
                class
            }
            // A list of sources of its entries' codec, held to its bound.
            Value::List { codec, bound, .. } => {
                let import = printed_import(imports, codec)
                    .expect("`unlowered` prints a list of a printed codec only");
                let class = list_class(struct_name, id);
                before.push_str(&bounded_list_class(
                    struct_name,
                    id,
                    &class,
                    bound,
                    &format!("{}Source", body_type(import)),
                ));
                class
            }
            _ => {
                let ty = l.type_name(&element.field.sce_type).into_owned();
                // A number the value states only while the content it measures
                // is absent (docs/adr/0058 §1): `null` while it is there, which
                // the encode then derives from it, and the decode gives back
                // `null` where it read any of it.
                if element.presence == Presence::Stated {
                    let ty = l.optional_of(&ty).expect("Kotlin wraps the present type");
                    let absent: Vec<String> = stated_contents(plan, id)
                        .iter()
                        .map(|content| format!("{} == null", l.codec_field_id(&content.id)))
                        .collect();
                    presented.insert(id.clone(), ty.clone());
                    held.insert(
                        id.clone(),
                        Held {
                            ty,
                            default: "null".to_string(),
                            made: format!(
                                "if ({}) {} else null",
                                absent.join(" && "),
                                l.codec_field_id(id)
                            ),
                        },
                    );
                    continue;
                }
                presented.insert(id.clone(), optional(ty));
                continue;
            }
        };
        inline = true;
        let local = l.codec_field_id(id);
        let (default, made) = if gated {
            (
                "null".to_string(),
                format!("{local}?.let {{ {class}.of(it) ?: return null }}"),
            )
        } else {
            (
                format!("{class}.fewest()"),
                format!("{class}.of({local}) ?: return null"),
            )
        };
        let ty = optional(class);
        presented.insert(id.clone(), ty.clone());
        held.insert(id.clone(), Held { ty, default, made });
    }

    // The tag a catch-all arm holds, within the tags no arm names.
    let unknown_tag = crate::forge::codec_totality::unknown_tag(plan.model).map(|unknown| {
        let class = unknown_tag_class(struct_name);
        before.push_str(&unknown_tag_declaration(struct_name, &class, &unknown, l));
        inline = true;
        class
    });

    // The source: what the encode reads, each element by the property a value
    // holds it in, `null` where a gate leaves it out, and the arm of the
    // variant the value holds, each arm a source of its codec.
    before.push_str(&format!(
        "/// The elements a value of [{struct_name}] presents, which its encode reads\n\
         /// (docs/adr/0081 §2): any object that presents them is a source of it, which\n\
         /// [{struct_name}.encodeSource] writes. The encode derives every other element\n\
         /// of the frame.\n\
         interface {struct_name}Source"
    ));
    if plan.elements.is_empty() && plan.arms.is_empty() {
        before.push('\n');
    } else {
        before.push_str(" {\n");
        for element in &plan.elements {
            let id = &element.field.id;
            before.push_str(&format!(
                "    val {}: {}\n",
                l.codec_field_id(id),
                presented[id]
            ));
        }
        if !plan.arms.is_empty() {
            before.push_str(&format!(
                "    val {VARIANT_PROPERTY}: {}\n",
                variant_class(struct_name)
            ));
        }
        before.push_str("}\n");
    }

    KotlinSource {
        before: before.trim_end_matches('\n').to_string(),
        encode: encode_source(plan, l, imports, struct_name),
        said_held: plan
            .said
            .iter()
            .map(|said| {
                format!(
                    "        if ({} != {}(this)) return CodecError.PresentIfMismatch\n",
                    input_name(&said.input.name),
                    said_function(&said.input.name)
                )
            })
            .collect(),
        held,
        inline,
        unknown_tag,
    }
}

/// The property a value presents the arm of its variant by, as the template's
/// data class holds it.
const VARIANT_PROPERTY: &str = "body";

/// The sealed class the Kotlin value of the codec whose data class is
/// `struct_name` holds the arm of its variant in, as the template declares it.
pub(crate) fn variant_class(struct_name: &str) -> String {
    format!("{struct_name}Variant")
}

/// The class of the sealed class of `struct_name`'s variant that holds the arm
/// whose codec is imported as `codec`, or the catch-all where `codec` is
/// `None`, as the template declares it ([`variant_arm_name`]).
pub(crate) fn arm_class(struct_name: &str, codec: Option<&str>) -> String {
    format!("{}.{}", variant_class(struct_name), variant_arm_name(codec))
}

/// The class the catch-all arm of the variant of the codec whose data class is
/// `struct_name` holds its tag in (docs/adr/0065 §6 R3), named as every backend
/// names it ([`super::rust_source::unknown_tag_type`]). The one place the
/// Kotlin name is made: the printer declares the class by it, the template
/// types the arm with it, and the conformance harness makes a value of it.
pub(crate) fn unknown_tag_class(struct_name: &str) -> String {
    super::rust_source::unknown_tag_type(struct_name)
}

/// The value class `class` the catch-all arm of `struct_name`'s variant holds
/// its tag in: a private constructor over the tag's type, and `of`, which
/// makes a value only of a tag the bits it is read from hold and no arm
/// names, so the catch-all a value holds is written and read back as the
/// catch-all (docs/adr/0065 §6 R3), and the encode refuses nothing of it.
fn unknown_tag_declaration(
    struct_name: &str,
    class: &str,
    unknown: &crate::forge::codec_totality::UnknownTag,
    l: &LangCtx,
) -> String {
    let ty = l.type_name(&unknown.ty).into_owned();
    let bits = CarrierBits::of(&unknown.ty);
    let viewed = bits.viewed("tag");
    // The tags the arms name, each run of consecutive ones a range.
    let known: Vec<String> = unknown
        .known_runs()
        .iter()
        .map(|&(first, last)| {
            if first == last {
                bits.literal(first)
            } else {
                format!("in {}..{}", bits.literal(first), bits.literal(last))
            }
        })
        .collect();
    let within = match unknown.max_below_type() {
        Some(max) => format!(
            "if ({viewed} <= {}) {class}(tag) else null",
            bits.literal(max)
        ),
        None => format!("{class}(tag)"),
    };
    let of = if known.is_empty() {
        format!(" =\n            {within}")
    } else {
        format!(
            " =\n            when ({viewed}) {{\n\
             \x20               {} -> null\n\
             \x20               else -> {within}\n\
             \x20           }}",
            known.join(", ")
        )
    };
    let fewest = (0..=unknown.max())
        .find(|tag| unknown.admits(*tag))
        .expect("a variant whose every tag an arm names has no reachable catch-all (codec/variant-arm-unreachable)");
    let fewest = match bits.back {
        Some(back) => format!("{}.{back}()", bits.literal(fewest)),
        None => bits.literal(fewest),
    };
    let wide = ulong_of("tag", &unknown.ty);
    format!(
        "/// The tag the catch-all arm of [{struct_name}]'s variant holds: one the bits\n\
         /// the tag is read from hold and no arm names, held to that where a value of\n\
         /// it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all a value\n\
         /// holds is written and read back as the catch-all.\n\
         @JvmInline\n\
         value class {class} private constructor(val tag: {ty}) {{\n\
         \x20   /// The tag as a `ULong`, the type an encode derives a tag in.\n\
         \x20   val {UNKNOWN_TAG_WIDE}: ULong\n\
         \x20       get() = {wide}\n\
         \n\
         \x20   companion object {{\n\
         \x20       /// `tag` as a value of the catch-all, or `null` where an arm names it or the\n\
         \x20       /// bits the tag is read from do not hold it.\n\
         \x20       fun of(tag: {ty}): {class}?{of}\n\
         \n\
         \x20       /// The least tag no arm names: what a catch-all made by default holds.\n\
         \x20       fun fewest(): {class} = {class}({fewest})\n\
         \x20   }}\n\
         }}\n\n"
    )
}

/// The class the Kotlin value of the codec whose data class is `struct_name`
/// holds its list `id` in, within its bound (docs/adr/0065 §6 R2). The one
/// place the name is made, as [`refinement_class`] is for a byte string.
pub(crate) fn list_class(struct_name: &str, id: &str) -> String {
    format!(
        "{struct_name}{}List",
        filters::to_pascal_case(id.to_string())
    )
}

/// The value class a list `id` of `struct_name` is held in, `class`, within
/// `bound`, of sources of its entries' codec, `entry`: a private constructor,
/// and `of`, which makes a value only of entries within the bound, copied into
/// a list of its own, so no list it was made of can grow it past its bound
/// after. A list of a least past zero is not printed, so a value made by
/// default holds no entry.
fn bounded_list_class(
    struct_name: &str,
    id: &str,
    class: &str,
    bound: &Bound,
    entry: &str,
) -> String {
    let (held, most, of) = match bound.most {
        Some(most) => (
            format!("0 to {most} entries"),
            format!(
                "\n\
                 \x20       /// The most entries a value holds.\n\
                 \x20       const val MOST: Int = {most}\n"
            ),
            format!(
                " =\n            if (entries.size <= MOST) {class}(entries.toList()) else null"
            ),
        ),
        None => (
            "any number of entries, the protocol stating no most".to_string(),
            String::new(),
            format!(" = {class}(entries.toList())"),
        ),
    };
    format!(
        "/// The `{id}` element of [{struct_name}], {held}, held to that bound where a\n\
         /// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is\n\
         /// private and [of] makes a value only of entries within the bound, which it\n\
         /// copies, so no list it was made of can grow it past its bound after.\n\
         @JvmInline\n\
         value class {class} private constructor(val entries: List<{entry}>) {{\n\
         \x20   companion object {{{most}\n\
         \x20       /// `entries` as a value of the bound, or `null` where they lie outside it.\n\
         \x20       fun of(entries: List<{entry}>): {class}?{of}\n\
         \n\
         \x20       /// No entry: what a value made by default holds.\n\
         \x20       fun fewest(): {class} = {class}(emptyList())\n\
         \x20   }}\n\
         }}\n\n"
    )
}

/// The value class a byte string or a text (`text`) `id` of `struct_name` is
/// held in, `class`, within `bound`: a private constructor, and `of`, which
/// makes a value only of bytes, or of a text whose UTF-8 bytes, within the
/// bound. A text no UTF-8 holds (one with a lone surrogate) is no value of it.
fn bounded_class(struct_name: &str, id: &str, class: &str, bound: &Bound, text: bool) -> String {
    let least = bound.least;
    let (unit, of_what, holder, size) = if text {
        ("UTF-8 bytes", "a text", "text: String", "size")
    } else {
        ("bytes", "bytes", "bytes: ByteArray", "bytes.size")
    };
    let held = match bound.most {
        Some(most) => format!("{least} to {most} {unit}"),
        None => format!("{least} or more {unit}, the protocol stating no most"),
    };
    let most = bound.most.map_or_else(String::new, |most| {
        format!(
            "\n\
             \x20       /// The most {unit} a value holds.\n\
             \x20       const val MOST: Int = {most}\n"
        )
    });
    // Only the comparisons the bound makes: a least of zero holds every count,
    // and no most none above it.
    let test = match (least, bound.most) {
        (0, None) => None,
        (0, Some(_)) => Some(format!("{size} <= MOST")),
        (_, None) => Some(format!("{size} >= LEAST")),
        (_, Some(_)) => Some(format!("{size} in LEAST..MOST")),
    };
    let argument = if text { "text" } else { "bytes" };
    let of = match (text, test) {
        (false, Some(test)) => format!(" =\n            if ({test}) {class}(bytes) else null"),
        (false, None) => format!(" = {class}(bytes)"),
        // A text's bound is counted in the bytes UTF-8 encodes it to, as every
        // backend counts a text's length on the wire.
        (true, Some(test)) => format!(
            " {{\n\
             \x20           val size = com.sce.forge.runtime.Utf8.size(text) ?: return null\n\
             \x20           return if ({test}) {class}(text) else null\n\
             \x20       }}"
        ),
        (true, None) => format!(
            " =\n            if (com.sce.forge.runtime.Utf8.size(text) != null) {class}(text) else null"
        ),
    };
    let fewest = if text {
        format!("{class}(\"\\u0000\".repeat(LEAST))")
    } else {
        format!("{class}(ByteArray(LEAST))")
    };
    format!(
        "/// The `{id}` element of [{struct_name}], {held}, held to that bound where a\n\
         /// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is\n\
         /// private and [of] makes a value only of {of_what} within the bound. The value it\n\
         /// holds cannot change after, so a value holds its bound for as long as it lives.\n\
         @JvmInline\n\
         value class {class} private constructor(val {holder}) {{\n\
         \x20   companion object {{\n\
         \x20       /// The fewest {unit} a value holds.\n\
         \x20       const val LEAST: Int = {least}\n\
         {most}\n\
         \x20       /// `{argument}` as a value of the bound, or `null` where it lies outside it.\n\
         \x20       fun of({holder}): {class}?{of}\n\
         \n\
         \x20       /// The fewest {unit} the bound holds, each zero: what a value made by\n\
         \x20       /// default holds.\n\
         \x20       fun fewest(): {class} = {fewest}\n\
         \x20   }}\n\
         }}\n\n"
    )
}

/// `count` converted to the Kotlin type of `ty`, the type of the field a
/// derived number goes into; `count` is an `Int`.
fn int_as(count: &str, ty: &SceType) -> String {
    let conversion = match ty {
        SceType::Uint8 => "toUByte",
        SceType::Uint16 => "toUShort",
        SceType::Uint32 => "toUInt",
        SceType::Uint64 => "toULong",
        SceType::Int8 => "toByte",
        SceType::Int16 => "toShort",
        SceType::Int64 => "toLong",
        // Already an `Int`, which a conversion would only repeat.
        _ => return count.to_string(),
    };
    if count.contains(' ') {
        format!("({count}).{conversion}()")
    } else {
        format!("{count}.{conversion}()")
    }
}

/// How the bits of a carrier of type `ty` are worked on: the unsigned type of
/// its width (`UInt`, or `ULong` for 64 bits) and the conversions into it and
/// back, each `None` where the carrier is that type already. Unsigned, so a
/// mask of the top bit is a literal of the type.
struct CarrierBits {
    /// Whether the carrier is worked on as a `ULong`.
    wide: bool,
    /// The conversion into the unsigned type.
    view: Option<&'static str>,
    /// The conversion back to the carrier's type.
    back: Option<&'static str>,
}

impl CarrierBits {
    fn of(ty: &SceType) -> Self {
        let (wide, view, back) = match ty {
            SceType::Uint8 => (false, Some("toUInt"), Some("toUByte")),
            SceType::Uint16 => (false, Some("toUInt"), Some("toUShort")),
            SceType::Uint32 => (false, None, None),
            SceType::Uint64 => (true, None, None),
            SceType::Int8 => (false, Some("toUInt"), Some("toByte")),
            SceType::Int16 => (false, Some("toUInt"), Some("toShort")),
            SceType::Int64 => (true, Some("toULong"), Some("toLong")),
            _ => (false, Some("toUInt"), Some("toInt")),
        };
        CarrierBits { wide, view, back }
    }

    /// `value` as the unsigned type.
    fn viewed(&self, value: &str) -> String {
        match self.view {
            Some(view) => format!("{value}.{view}()"),
            None => value.to_string(),
        }
    }

    /// `n` as a literal of the unsigned type.
    fn literal(&self, n: u64) -> String {
        if self.wide {
            format!("0x{n:X}uL")
        } else {
            format!("0x{n:X}u")
        }
    }
}

/// `value`, the local an element of bytes or of a text (`text`) is read into,
/// as the bytes it is written as.
fn wire_bytes(value: &str, text: bool) -> String {
    if text {
        format!("{value}.text.encodeToByteArray()")
    } else {
        format!("{value}.bytes")
    }
}

/// One write of the encode at the body's indentation, its sink's failure
/// answered at once.
fn written(call: &str) -> String {
    format!("{call}?.let {{ return it }}\n")
}

/// Each line of `body` one level deeper.
fn indented(body: &str) -> String {
    body.lines()
        .map(|line| {
            if line.is_empty() {
                "\n".to_string()
            } else {
                format!("    {line}\n")
            }
        })
        .collect()
}

/// The encode of `struct_name` over any source of it, a function of the value's
/// companion object: each element read once off the source into a local, each
/// derived number made from what it measures and each derived flag folded into
/// its carrier, then the writes in wire order. A function of the companion
/// object and not a method of the value, so it writes any source, and a
/// property of the value cannot take its name.
fn encode_source(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> String {
    let numbers = derived_numbers(plan);
    // A number the value states while its content is absent is derived where
    // the content is there; every other is derived always.
    let (stated, derived): (Vec<&String>, Vec<&String>) = numbers
        .iter()
        .partition(|id| element_of(plan, id).is_some());
    let named = |ids: &[&String]| {
        ids.iter()
            .map(|id| format!("`{}`", l.codec_field_id(id)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut derives = String::new();
    if !derived.is_empty() {
        derives.push_str(&format!(
            " It derives {} from what each measures, and never reads it off the\n/// source.",
            named(&derived)
        ));
    }
    if !stated.is_empty() {
        derives.push_str(&format!(
            " It derives {} where what it measures is there, and reads what\n/// the source \
             states where none is.",
            named(&stated)
        ));
    }
    // The elements the body reads, which are read off the source once, before
    // the first write.
    let reads: RefCell<BTreeSet<String>> = RefCell::new(BTreeSet::new());
    let element = |id: &str| -> String {
        reads.borrow_mut().insert(id.to_string());
        format!("_e_{}", l.codec_field_id(id))
    };
    // The flag inputs the body reads, each a parameter of the encode.
    let inputs_read: RefCell<BTreeSet<String>> = RefCell::new(BTreeSet::new());
    let input = |name: &str| -> String {
        inputs_read.borrow_mut().insert(name.to_string());
        input_name(name)
    };
    // Whether the body reads the arm of the variant the value holds, which is
    // read off the source once, before the first write, as `_r_body`.
    let variant_read = Cell::new(false);
    let variant = || -> &'static str {
        variant_read.set(true);
        VARIANT_LOCAL
    };
    let is_text = |field: &CodecField| field.sce_type == SceType::String;
    // The data class of the codec whose companion's `encodeSource` writes the
    // body `field`, or each entry of the list `field`.
    let body_writer = |field: &CodecField| -> String {
        let import = printed_import(imports, held_alias(plan, field))
            .expect("`unlowered` prints a codec it holds printed");
        body_type(import).to_string()
    };
    // A codec it holds can refuse the value as it is written, so the frame is
    // written whole first and handed to the caller's sink only once it is, as
    // a count over a sink that keeps nothing precedes the write in the other
    // printers (docs/adr/0081 §1): a refusal leaves the caller's sink as it
    // was. Every write then goes to the staged sink, `_s`.
    let mut staged = false;
    for r in plan.totality.refusals() {
        if r.kind == RefusalKind::Child {
            plan.totality.require(RefusalKind::Child, &r.element);
            staged = true;
        }
    }
    let sink = if staged { "_s" } else { "w" };
    // The bodies encoded into a buffer first, by field id: those whose length a
    // number measures.
    let mut buffered: BTreeSet<String> = BTreeSet::new();
    let mut body = String::new();
    // Each input the value says (docs/adr/0065 §6 R4a) that a gate or a check
    // of the codec reads, read off what it holds under the name the gate reads
    // it by; one none reads is left to the parent that asks for it.
    for said in plan.said.iter().filter(|said| said.read) {
        body.push_str(&format!(
            "val {} = {}\n",
            input_name(&said.input.name),
            said_value(plan, imports, &said.input.name, &element)
        ));
    }
    // A measure of a derived number: what a body says, by its codec's
    // function, or what the source holds.
    let measure_text = |measure: &Measure<'_>| -> MeasureText {
        match &measure.of {
            Measured::Says {
                body: held, input, ..
            } => {
                let content = element(&held.id);
                MeasureText {
                    size: format!(
                        "{}.{}({content}).toInt()",
                        body_writer(held),
                        said_function(input)
                    ),
                    present: measure.gated.then(|| format!("{content} != null")),
                }
            }
            _ => plain_measure(&element, measure),
        }
    };
    // Each flag the encode derives, by its carrier: the statements that fold it
    // into the carrier's bits, `_u_<carrier>`, in place of the bits the value
    // presents.
    let mut folds: BTreeMap<String, Vec<String>> = BTreeMap::new();
    // The numbers that measure a body's frame, made once the carriers its
    // bodies may be handed are as the encode writes them.
    let mut encoded: Vec<(&ElementId, &Number<'_>)> = Vec::new();
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            (element_id, Derive::Number(number))
                if number
                    .measures
                    .iter()
                    .any(|m| matches!(m.of, Measured::Encoded { .. })) =>
            {
                encoded.push((element_id, number));
            }
            (element_id, Derive::Number(number)) => {
                let measured: Vec<MeasureText> =
                    number.measures.iter().map(&measure_text).collect();
                if let Some((carrier, statement)) = derive_number(
                    plan,
                    l,
                    element_id,
                    &number.into,
                    &measured,
                    &element,
                    &mut body,
                ) {
                    folds.entry(carrier).or_default().push(statement);
                }
            }
            (ElementId::Flag { field, flag }, Derive::Presence(presence)) => {
                let (gated, when_clear) =
                    presence.of.first().expect("a presence flag gates a field");
                let def = flag_def(plan, field, flag);
                let bits = CarrierBits::of(&carrier_field(plan, field).sce_type);
                let mask = bits.literal(((1u64 << def.width) - 1) << def.bit);
                let test = if *when_clear { "==" } else { "!=" };
                let u = format!("_u_{}", l.codec_field_id(field));
                folds.entry(field.clone()).or_default().push(format!(
                    "{u} = ({u} and {mask}.inv()) or (if ({} {test} null) {mask} else {})\n",
                    element(&gated.id),
                    bits.literal(0)
                ));
            }
            // The tag of the arm the value holds: the value the document gives
            // a named arm, and the tag the catch-all holds.
            (element_id, Derive::VariantTag) => {
                body.push_str(&arm_tag(plan, struct_name, variant()));
                match element_id {
                    ElementId::Field(id) => {
                        let field = carrier_field(plan, id);
                        body.push_str(&format!(
                            "val _d_{} = {}\n",
                            l.codec_field_id(id),
                            ulong_as(ARM_TAG_LOCAL, &field.sce_type)
                        ));
                    }
                    // Masked to the flag's bits, which leave the carrier's
                    // other bits as the value presents them.
                    ElementId::Flag { field, flag } => {
                        folds.entry(field.clone()).or_default().push(fold_bits(
                            plan,
                            l,
                            field,
                            flag,
                            ARM_TAG_LOCAL,
                        ));
                    }
                    ElementId::Variant => unreachable!(
                        "a variant whose caller hands its tag derives none: the caller writes it"
                    ),
                }
            }
            // The tag of the arm each embedded body's variant holds, masked to
            // the flag's bits, the catch-all's the tag it holds; nothing where
            // a gate leaves the body out.
            (ElementId::Flag { field, flag }, Derive::ParentTag(tag)) => {
                let local = format!("_p_{}", derived_local(l, &derived.element));
                body.push_str(&format!("var {local}: ULong = 0x0uL\n"));
                for term in &tag.terms {
                    let import = printed_import(imports, &term.codec)
                        .expect("`unlowered` prints a body of a printed codec only");
                    let held = body_type(import);
                    let content = element(&term.body.id);
                    let mut cases = String::new();
                    for arm in &term.arms {
                        cases.push_str(&format!(
                            "    is {} -> {local} = {local} or 0x{:X}uL\n",
                            arm_class(held, Some(arm.codec.as_str())),
                            arm.tag
                        ));
                    }
                    if term.catch_all {
                        cases.push_str(&format!(
                            "    is {} -> {local} = {local} or (_v.tag.{UNKNOWN_TAG_WIDE} and 0x{:X}uL)\n",
                            arm_class(held, None),
                            term.mask
                        ));
                    }
                    let subject = if term.catch_all {
                        format!("val _v = {content}.{VARIANT_PROPERTY}")
                    } else {
                        format!("{content}.{VARIANT_PROPERTY}")
                    };
                    let switch = format!("when ({subject}) {{\n{cases}}}\n");
                    let gated =
                        element_of(plan, &term.body.id).is_some_and(|e| e.presence.optional());
                    if gated {
                        body.push_str(&format!(
                            "if ({content} != null) {{\n{}}}\n",
                            indented(&switch)
                        ));
                    } else {
                        body.push_str(&switch);
                    }
                }
                folds
                    .entry(field.clone())
                    .or_default()
                    .push(fold_bits(plan, l, field, flag, &local));
            }
            _ => unreachable!("`unlowered` prints no other derivation"),
        }
    }
    // Each carrier as the encode writes it: its flags the value presents, and
    // those the encode derives folded in, in place of the value's, worked on as
    // the unsigned type of its width and read back as its own type.
    for (carrier, carried) in &folds {
        let bits = CarrierBits::of(&carrier_field(plan, carrier).sce_type);
        let name = l.codec_field_id(carrier);
        body.push_str(&format!(
            "var _u_{name} = {}\n",
            bits.viewed(&element(carrier))
        ));
        for statement in carried {
            body.push_str(statement);
        }
        let back = match bits.back {
            Some(back) => format!("_u_{name}.{back}()"),
            None => format!("_u_{name}"),
        };
        body.push_str(&format!("val _c_{name} = {back}\n"));
    }
    let wholes = derived_fields(plan);
    let read = |f: &CodecField| -> String {
        let name = l.codec_field_id(&f.id);
        if wholes.contains(&f.id) {
            format!("_d_{name}")
        } else if folds.contains_key(&f.id) {
            format!("_c_{name}")
        } else {
            element(&f.id)
        }
    };
    // A gate, tested on the carriers as the encode writes them.
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
                } => {
                    let bits = CarrierBits::of(&carrier.sce_type);
                    format!(
                        "(({} shr {bit}) and {}) {} {}",
                        bits.viewed(&read(carrier)),
                        bits.literal(1),
                        if *negate { "==" } else { "!=" },
                        bits.literal(0)
                    )
                }
                // An input the encode is handed.
                Clause::Input { name, negate } => format!(
                    "{}.toUInt() {} 0u",
                    input(name),
                    if *negate { "==" } else { "!=" }
                ),
                Clause::Member { .. } => {
                    unreachable!("`unlowered` prints no gate on a chain's entries")
                }
            })
            .collect::<Vec<_>>()
            .join(" || ")
    };
    // The arguments after the sink the codec imported as `alias` is handed:
    // for each input it takes and does not say, the bits of a carrier as the
    // encode writes it, or an input of this codec's own passed on.
    let handed = |alias: &str| -> String {
        plan.handed
            .get(alias)
            .map(|inputs| {
                inputs
                    .iter()
                    .map(|h| match &h.from {
                        Some(HandedFrom::Carrier {
                            carrier,
                            bit,
                            width,
                        }) => {
                            let bits = CarrierBits::of(&carrier.sce_type);
                            let mask = if *width >= 64 {
                                u64::MAX
                            } else {
                                (1u64 << width) - 1
                            };
                            format!(
                                ", (({} shr {bit}) and {}).toUByte()",
                                bits.viewed(&read(carrier)),
                                bits.literal(mask)
                            )
                        }
                        Some(HandedFrom::Input { name }) => format!(", {}", input(name)),
                        None => unreachable!(
                            "`unlowered` prints no codec handed an input from the entry before"
                        ),
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    // Each number that measures a body's frame: the body encoded once into a
    // buffer the write of the body then writes, `null` where it is absent. A
    // body whose encode refuses answers its refusal here, before the first
    // byte of this frame.
    for (element_id, number) in encoded {
        let mut measured: Vec<MeasureText> = Vec::new();
        for measure in &number.measures {
            let Measured::Encoded { body: held, .. } = &measure.of else {
                unreachable!("{}", ENCODED_ALONE)
            };
            let content = element(&held.id);
            let name = l.codec_field_id(&held.id);
            let (list, buffer) = (format!("_l_{name}"), format!("_b_{name}"));
            if buffered.insert(held.id.clone()) {
                let encode = format!(
                    "val {list} = mutableListOf<Byte>()\n{}",
                    written(&format!(
                        "{}.encodeSource({content}, MutableListSink({list}){})",
                        body_writer(held),
                        handed(held_alias(plan, held))
                    ))
                );
                if measure.gated {
                    body.push_str(&format!(
                        "var {buffer}: ByteArray? = null\n\
                         if ({content} != null) {{\n\
                         {}    {buffer} = {list}.toByteArray()\n\
                         }}\n",
                        indented(&encode)
                    ));
                } else {
                    body.push_str(&format!("{encode}val {buffer} = {list}.toByteArray()\n"));
                }
            }
            measured.push(MeasureText {
                size: format!("{buffer}.size"),
                present: measure.gated.then(|| format!("{buffer} != null")),
            });
        }
        if derive_number(
            plan,
            l,
            element_id,
            &number.into,
            &measured,
            &element,
            &mut body,
        )
        .is_some()
        {
            unreachable!("{}", ENCODED_INTO_FIELD);
        }
    }
    // Each gate the encode cannot derive the presence from holds the field it
    // gates, before the first byte (docs/adr/0065 §2).
    for field in &plan.gate_checks {
        body.push_str(&format!(
            "if (({}) != ({} != null)) {{\n    return {}\n}}\n",
            gate_test(field),
            element(&field.id),
            refusal(plan, RefusalKind::PresentIfMismatch, &field.id)
        ));
    }
    // Each input a held codec says, held to what this codec hands it from
    // elsewhere, before the first byte: a value whose body or entry says
    // otherwise has no frame (docs/adr/0065 §6 R4a).
    for check in &plan.input_checks {
        let against = match &check.against {
            Against::Carrier { carrier, bit } => {
                let bits = CarrierBits::of(&carrier.sce_type);
                format!(
                    "(({} shr {bit}) and {}).toUInt()",
                    bits.viewed(&read(carrier)),
                    bits.literal(1)
                )
            }
            Against::Input { name } => format!("{}.toUInt()", input(name)),
            Against::PreviousEntry { .. } => {
                unreachable!("`unlowered` prints no check against the entry before")
            }
        };
        let import = printed_import(imports, &check.codec)
            .expect("`unlowered` prints a codec it holds printed");
        let says = |held: &str| {
            format!(
                "{}.{}({held}).toUInt() != {against}",
                body_type(import),
                said_function(&check.input)
            )
        };
        let fail = refusal(plan, RefusalKind::PresentIfMismatch, &check.at);
        match &check.site {
            CheckSite::Embed { body: held } => {
                let content = element(&held.id);
                let test = if element_of(plan, &held.id).is_some_and(|e| e.presence.optional()) {
                    format!("{content} != null && {}", says(&content))
                } else {
                    says(&content)
                };
                body.push_str(&format!("if ({test}) {{\n    return {fail}\n}}\n"));
            }
            CheckSite::List { list } => {
                let content = element(&list.id);
                // A list a gate leaves out holds no entry to check.
                let entries = if element_of(plan, &list.id).is_some_and(|e| e.presence.optional()) {
                    format!("({content}?.entries ?: emptyList())")
                } else {
                    format!("{content}.entries")
                };
                let walk = format!(
                    "for (_entry in {entries}) {{\n    if ({}) {{\n        return {fail}\n    }}\n}}\n",
                    says("_entry")
                );
                body.push_str(&walk);
            }
            CheckSite::Arm { .. } => unreachable!("`unlowered` prints no variant"),
        }
    }
    if staged {
        body.push_str("val _f = mutableListOf<Byte>()\nval _s = MutableListSink(_f)\n");
    }
    let endian = plan.model.default_endian;
    for write in &plan.program {
        match write {
            Write::Whole(field) => {
                for byte in encode_whole_field_read_as(field, endian, &read(field), l) {
                    body.push_str(&written(&format!("{sink}.writeU8({byte})")));
                }
            }
            Write::Byte { pieces, .. } => {
                body.push_str(&written(&format!(
                    "{sink}.writeU8({})",
                    encode_byte_read_by(pieces, l, &read)
                )));
            }
            Write::Tail { field, pad } => {
                // A field the layout aligns follows zeros up to its offset.
                if *pad {
                    body.push_str(&format!(
                        "while ({sink}.position() < {}) {{\n    {}}}\n",
                        field.byte_offset,
                        written(&format!("{sink}.writeU8(0)"))
                    ));
                }
                body.push_str(&written(&format!(
                    "{sink}.writeBytes({})",
                    wire_bytes(&read(field), is_text(field))
                )));
            }
            Write::Streamed { field, guard, .. } => {
                let write = match &field.bit_size {
                    BitSize::Fixed { .. } => {
                        encode_whole_field_read_as(field, endian, &read(field), l)
                            .iter()
                            .map(|byte| written(&format!("{sink}.writeU8({byte})")))
                            .collect::<String>()
                    }
                    // The runtime's writer of a VLE of the field's width,
                    // which takes the value as the unsigned type of it.
                    BitSize::Vle { width_bits } => written(&format!(
                        "{sink}.writeVleU{width_bits}({})",
                        l.codec_carrier_expr(&field.sce_type, &read(field))
                    )),
                    BitSize::Tail | BitSize::LengthRef => written(&format!(
                        "{sink}.writeBytes({})",
                        wire_bytes(&read(field), is_text(field))
                    )),
                    // A body: the buffer its length was measured over, or its
                    // codec's encode over the source it is presented as.
                    BitSize::Embed if buffered.contains(&field.id) => written(&format!(
                        "{sink}.writeBytes(_b_{})",
                        l.codec_field_id(&field.id)
                    )),
                    BitSize::Embed => written(&format!(
                        "{}.encodeSource({}, {sink}{})",
                        body_writer(field),
                        read(field),
                        handed(held_alias(plan, field))
                    )),
                    // A list or a chain walked entry by entry, each written by
                    // its codec's encode over the source the list holds.
                    BitSize::Repeat { .. } | BitSize::TlvChain { .. } => format!(
                        "for (_entry in {}.entries) {{\n    {}}}\n",
                        read(field),
                        written(&format!(
                            "{}.encodeSource(_entry, {sink}{})",
                            body_writer(field),
                            handed(held_alias(plan, field))
                        ))
                    ),
                };
                // What says whether the value holds the field: the local it
                // was read into, or the buffer a body was encoded into, which
                // is `null` where the body is absent.
                let held = if buffered.contains(&field.id) {
                    format!("_b_{}", l.codec_field_id(&field.id))
                } else {
                    read(field)
                };
                body.push_str(&match guard {
                    Guard::Always => write,
                    // Where the value holds it: the local it is tested by is
                    // not `null`, which narrows it for the write.
                    Guard::Present => {
                        format!("if ({held} != null) {{\n{}}}\n", indented(&write))
                    }
                    // Derived whole and gated: where its gate holds, as the
                    // carriers the encode writes say.
                    Guard::Gate => {
                        format!("if ({}) {{\n{}}}\n", gate_test(field), indented(&write))
                    }
                });
            }
            // The arm the value holds, written by its codec's encode over the
            // source the arm holds, handed what the arm's codec is handed.
            Write::Variant => {
                let mut cases = String::new();
                for arm in &plan.arms {
                    let import = printed_import(imports, arm.codec)
                        .expect("`unlowered` prints a variant of printed arms only");
                    cases.push_str(&format!(
                        "    is {} -> {}",
                        arm_class(struct_name, arm.tag.map(|_| arm.codec)),
                        written(&format!(
                            "{}.encodeSource(_b.body, {sink}{})",
                            body_type(import),
                            handed(arm.codec)
                        ))
                    ));
                }
                body.push_str(&format!("when (val _b = {}) {{\n{cases}}}\n", variant()));
            }
        }
    }
    // The staged frame, handed to the caller's sink whole.
    if staged {
        body.push_str(&written("w.writeBytes(_f.toByteArray())"));
    }
    body.push_str("return null\n");
    // Every refusal the totality analysis names is one the encode answers
    // (docs/adr/0065 §2).
    plan.totality.assert_all_written(struct_name);

    // Each element the body reads, once, in declaration order.
    let reads = reads.into_inner();
    let mut bound = String::new();
    for element in &plan.elements {
        if reads.contains(&element.field.id) {
            let name = l.codec_field_id(&element.field.id);
            bound.push_str(&format!("val _e_{name} = src.{name}\n"));
        }
    }
    // The arm of the variant, after the elements, as the source declares it.
    if variant_read.get() {
        bound.push_str(&format!("val {VARIANT_LOCAL} = src.{VARIANT_PROPERTY}\n"));
    }
    // A codec that writes nothing reads no element and touches no sink, and an
    // input the encode is handed may be one only its decode reads.
    let inputs_read = inputs_read.into_inner();
    let suppress = if (reads.is_empty() && !variant_read.get())
        || plan
            .given
            .iter()
            .any(|given| !inputs_read.contains(&given.name))
    {
        "@Suppress(\"UNUSED_PARAMETER\")\n"
    } else {
        ""
    };
    // What the encode refuses, each refusal the runtime's error object:
    // nothing, or what the plan's totality names, before the first byte.
    let refuses = if plan.totality.is_total() {
        " It refuses no value: it answers `null`, or\n/// [CodecError.BufferOverflow] from \
         a bounded sink with no room left."
            .to_string()
    } else {
        let mut answered: Vec<String> = plan
            .totality
            .refusals()
            .iter()
            .map(|r| match r.kind.error_variant() {
                Some(variant) => format!("[CodecError.{variant}]"),
                None => "what a codec it holds answers".to_string(),
            })
            .collect();
        answered.sort();
        answered.dedup();
        format!(
            " It answers\n/// {} for a value no frame carries,\n/// before it writes a byte \
             to `w`; otherwise `null`, or [CodecError.BufferOverflow]\n/// from a bounded \
             sink with no room left.",
            answered.join(", ")
        )
    };
    // The flag inputs the encode is handed, those the value does not say,
    // after the sink, as the decode is handed them after the cursor.
    let given: String = plan
        .given
        .iter()
        .map(|input| format!(", {}: UByte", input_name(&input.name)))
        .collect();
    let mut text = format!(
        "/// Write any source of [{struct_name}] into `w`: each element read once off\n\
         /// `src` and written where the codec's encode plan lays it out\n\
         /// (docs/adr/0081).{derives}{refuses}\n\
         {suppress}\
         fun encodeSource(src: {struct_name}Source, w: SceSink{given}): CodecError? {{\n\
         {}\
         }}",
        indented(&format!("{bound}{body}"))
    );
    // Each input the value says, read off any source of it: what a parent that
    // holds this codec derives the flag it binds to the input from, or holds
    // what it hands the codec to (docs/adr/0065 §6 R4a).
    for said in &plan.said {
        let value = said_value(plan, imports, &said.input.name, &|id: &str| {
            format!("src.{}", l.codec_field_id(id))
        });
        text.push_str(&format!(
            "\n\n/// The flag input `{}` a source of [{struct_name}] says, which its encode\n\
             /// is not handed (docs/adr/0065 §6 R4a).\n\
             fun {}(src: {struct_name}Source): UByte =\n    {value}",
            said.input.name,
            said_function(&said.input.name)
        ));
    }
    indented(&indented(&text))
        .trim_end_matches('\n')
        .to_string()
}

/// The local the encode reads the arm of the variant the value holds into.
const VARIANT_LOCAL: &str = "_r_body";

/// The local the encode derives the tag of the arm the value holds into, a
/// `ULong`.
const ARM_TAG_LOCAL: &str = "_t_body";

/// The property of a catch-all's tag class that answers the tag as a `ULong`.
const UNKNOWN_TAG_WIDE: &str = "wide";

/// `value`, of the unsigned integer type `ty`, as a `ULong`.
fn ulong_of(value: &str, ty: &SceType) -> String {
    match ty {
        SceType::Uint64 => value.to_string(),
        _ => format!("{value}.toULong()"),
    }
}

/// `value`, a `ULong`, as the unsigned integer type `ty`.
fn ulong_as(value: &str, ty: &SceType) -> String {
    let conversion = match ty {
        SceType::Uint8 => "toUByte",
        SceType::Uint16 => "toUShort",
        SceType::Uint32 => "toUInt",
        SceType::Uint64 => return value.to_string(),
        _ => unreachable!("`unlowered` prints a variant's tag held in an unsigned field only"),
    };
    format!("{value}.{conversion}()")
}

/// The statement that derives the tag of the arm the value of `plan`, whose
/// data class is `struct_name`, holds, read as `variant`, into
/// [`ARM_TAG_LOCAL`]: the value the document gives a named arm, and the tag the
/// catch-all holds. The `when` is exhaustive over the sealed class.
fn arm_tag(plan: &EncodePlan<'_>, struct_name: &str, variant: &str) -> String {
    let mut cases = String::new();
    for arm in &plan.arms {
        let class = arm_class(struct_name, arm.tag.map(|_| arm.codec));
        cases.push_str(&match arm.tag {
            Some(tag) => format!("    is {class} -> 0x{tag:X}uL\n"),
            None => format!("    is {class} -> _b.tag.{UNKNOWN_TAG_WIDE}\n"),
        });
    }
    // The arm is named only where the catch-all reads its tag off it.
    let subject = if plan.arms.iter().any(|arm| arm.tag.is_none()) {
        format!("val _b = {variant}")
    } else {
        variant.to_string()
    };
    format!("val {ARM_TAG_LOCAL}: ULong = when ({subject}) {{\n{cases}}}\n")
}

/// The statement that folds `bits`, a `ULong`, masked to the bits of the flag
/// `flag` of the carrier `carrier` of `plan`, into the carrier's bits as the
/// encode writes it, `_u_<carrier>`, in place of the bits the value presents.
fn fold_bits(plan: &EncodePlan<'_>, l: &LangCtx, carrier: &str, flag: &str, bits: &str) -> String {
    let def = flag_def(plan, carrier, flag);
    let worked = CarrierBits::of(&carrier_field(plan, carrier).sce_type);
    let width_mask = if def.width >= 64 {
        u64::MAX
    } else {
        (1u64 << def.width) - 1
    };
    let mask = worked.literal(width_mask << def.bit);
    let u = format!("_u_{}", l.codec_field_id(carrier));
    let masked = format!("({bits} and 0x{width_mask:X}uL)");
    let number = if worked.wide {
        masked
    } else {
        format!("{masked}.toUInt()")
    };
    format!(
        "{u} = ({u} and {mask}.inv()) or ({number} shl {})\n",
        def.bit
    )
}

/// The import alias of the codec the body `field`, or each entry of the list
/// `field`, of `plan` is a value of.
fn held_alias<'p>(plan: &'p EncodePlan<'_>, field: &CodecField) -> &'p str {
    match element_of(plan, &field.id).map(|e| &e.value) {
        Some(Value::Body { codec } | Value::List { codec, .. }) => codec,
        _ => unreachable!("`{}` is a body or a list the value presents", field.id),
    }
}

/// The flag input `input` the value of `plan` says, as a `UByte` read through
/// `read`, which names each element it reads: set exactly where the field
/// gated on it alone is there (clear, for one gated on its clear state), or
/// what the body always there it hands the input on to says, read by that
/// body's codec's function (docs/adr/0065 §6 R4a).
fn said_value(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
    input: &str,
    read: &dyn Fn(&str) -> String,
) -> String {
    match &plan.roles.inputs[input] {
        InputSource::Gate { field, when_clear } => {
            let (set, clear) = if *when_clear { (0, 1) } else { (1, 0) };
            format!(
                "(if ({} != null) {set} else {clear}).toUByte()",
                read(field)
            )
        }
        InputSource::Forward {
            embed,
            alias,
            input: held,
            partly: false,
        } => {
            let import = printed_import(imports, alias)
                .expect("`unlowered` prints a codec it holds printed");
            format!(
                "{}.{}({})",
                body_type(import),
                said_function(held),
                read(embed)
            )
        }
        _ => unreachable!("`unlowered` prints an input a gated field or a body says only"),
    }
}

/// The field `id` of `plan`, a carrier a derived flag is folded into.
fn carrier_field<'m>(plan: &EncodePlan<'m>, id: &str) -> &'m CodecField {
    plan.model
        .fields
        .iter()
        .find(|f| f.id == id)
        .expect("a derived flag is of a field of the codec")
}

/// The flag `flag` of the carrier `carrier` of `plan`.
fn flag_def<'m>(plan: &EncodePlan<'m>, carrier: &str, flag: &str) -> &'m FlagDef {
    carrier_field(plan, carrier)
        .flags
        .iter()
        .find(|d| d.name == flag)
        .expect("the parser found the flag a derivation names")
}

/// The Kotlin name of the flag input `name`: the parameter the encode and the
/// decode are handed it by.
fn input_name(name: &str) -> String {
    filters::to_camel_case(name.to_string())
}

/// What the encode answers where it refuses a value with `kind` at `at`, the
/// runtime's error object of the refusal's name, which the totality analysis
/// names (`Totality::require`, docs/adr/0065 §2).
fn refusal(plan: &EncodePlan<'_>, kind: RefusalKind, at: &str) -> String {
    plan.totality.require(kind, at).replace("::", ".")
}

/// Why a number that measures a body's frame measures nothing else: the roles
/// refuse declarations of two kinds that name one element as ambiguous.
const ENCODED_ALONE: &str = "a number of encoded lengths measures nothing else \
     (codec_roles::classify)";

/// Why an encoded length is held in a field: `length-from` names a field
/// declared before the embed, never a flag (`validate_codec_embed_length_from`).
const ENCODED_INTO_FIELD: &str = "length-from names a field declared before the embed, never a \
     flag (validate_codec_embed_length_from)";

/// Why a length the wire holds less a constant is never past its content: a
/// length-ref field's bound takes its fewest bytes from that constant, so the
/// class of its bytes holds at least what the wire takes off
/// (`codec_totality::held_to_offset`, docs/adr/0077 §3), and the analysis
/// names no refusal at the content.
const LESS_IS_HELD: &str = "a length-ref's bound holds at least the bytes its length-arith takes \
     off (LengthBound::of), so the length is never negative";

/// One measure of a derived number as the encode reads it.
struct MeasureText {
    /// The number, an `Int`, which is read only where the measure is there.
    size: String,
    /// Where the measure may be absent, the test that it is there, which
    /// narrows the local the number reads.
    present: Option<String>,
}

/// A measure of a derived number that reads the source alone: the bytes of a
/// byte string or a text, less or more its `sce:length-arith`, or the entries
/// of a list, each tested present where a gate may leave it out. `element`
/// names the local each element is read into.
fn plain_measure(element: &dyn Fn(&str) -> String, measure: &Measure<'_>) -> MeasureText {
    let (content, size) = match &measure.of {
        Measured::Bytes { field, offset } => {
            let arith = match offset {
                Offset::None => String::new(),
                Offset::More { by } => format!(" + {by}"),
                Offset::Less { by, held: true } => format!(" - {by}"),
                Offset::Less { held: false, .. } => {
                    unreachable!("`{}`: {}", field.id, LESS_IS_HELD)
                }
            };
            let content = element(&field.id);
            let size = format!(
                "{}.size{arith}",
                wire_bytes(&content, field.sce_type == SceType::String)
            );
            (content, size)
        }
        // How many entries the list holds, within its bound.
        Measured::Entries { list } => {
            let content = element(&list.id);
            let size = format!("{content}.entries.size");
            (content, size)
        }
        _ => unreachable!("`unlowered` prints a length or a count only"),
    };
    MeasureText {
        size,
        present: measure.gated.then(|| format!("{content} != null")),
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
            "{}_{}",
            l.codec_field_id(field),
            filters::to_snake_case(flag.clone())
        ),
        ElementId::Variant => unreachable!("the plan derives nothing into the variant"),
    }
}

/// The statements, written into `out`, that make the derived number of
/// `element_id` from what it measures, as the Rust encode's `combine` makes it
/// (docs/adr/0081 §1): the first measure always there fixes it, and every
/// other one there must say the same, or the value is refused as
/// `InconsistentSource`; where every measure may be absent, it is the first
/// one there. A field holds it as `_d_<field>`, refused as `LengthOutOfRange`
/// where it may not fit; where nothing it measures is there, the field holds
/// zero or what the value states, read by `element`. A number held in a flag
/// is answered as the carrier and the statement that folds it into the
/// carrier's bits, `_u_<carrier>`.
fn derive_number(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    element_id: &ElementId,
    into: &NumberInto<'_>,
    measures: &[MeasureText],
    element: &dyn Fn(&str) -> String,
    out: &mut String,
) -> Option<(String, String)> {
    let named = element_id.to_string();
    let disagree = || {
        format!(
            "return {}",
            refusal(plan, RefusalKind::InconsistentSource, &named)
        )
    };
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
                        "if ({} != {}) {{\n    {}\n}}\n",
                        other.size,
                        anchor.size,
                        disagree()
                    ));
                }
                for g in &gated {
                    out.push_str(&format!(
                        "if ({} && {} != {}) {{\n    {}\n}}\n",
                        there(g),
                        g.size,
                        anchor.size,
                        disagree()
                    ));
                }
                (anchor.size.clone(), None)
            } else {
                // The first one there, `null` where none is.
                let first = format!("_f_{}", derived_local(l, element_id));
                out.push_str(&format!("var {first}: Int? = null\n"));
                for (i, g) in gated.iter().enumerate() {
                    let keyword = if i == 0 { "if" } else { "} else if" };
                    out.push_str(&format!(
                        "{keyword} ({}) {{\n    {first} = {}\n",
                        there(g),
                        g.size
                    ));
                }
                out.push_str("}\n");
                for g in gated.iter().skip(1) {
                    out.push_str(&format!(
                        "if ({} && {} != {first}) {{\n    {}\n}}\n",
                        there(g),
                        g.size,
                        disagree()
                    ));
                }
                let found = format!("{first} != null");
                (first, Some(found))
            }
        }
    };
    let (field, narrow, absent) = match (element_id, into) {
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
            let bits = CarrierBits::of(&carrier_field(plan, field).sce_type);
            let width_mask = if def.width >= 64 {
                u64::MAX
            } else {
                (1u64 << def.width) - 1
            };
            let mask = bits.literal(width_mask << def.bit);
            let u = format!("_u_{}", l.codec_field_id(field));
            let mut set = String::new();
            if plan.totality.refuses(RefusalKind::LengthOutOfRange, &named) {
                set.push_str(&format!(
                    "if ({size} > 0x{width_mask:X}) {{\n    return {}\n}}\n",
                    refusal(plan, RefusalKind::LengthOutOfRange, &named)
                ));
            }
            let number = if bits.wide {
                format!("({size}).toULong()")
            } else {
                format!("({size}).toUInt()")
            };
            set.push_str(&format!(
                "{u} = ({u} and {mask}.inv()) or ({number} shl {})\n",
                def.bit
            ));
            let statement = match &present {
                None => set,
                Some(there) => format!("if ({there}) {{\n{}}}\n", indented(&set)),
            };
            return Some((field.clone(), statement));
        }
        _ => unreachable!("a number goes into the field or the flag it names"),
    };
    let name = l.codec_field_id(&field.id);
    // A number past what the field holds is no frame (docs/adr/0065 §5). The
    // number is an `Int`, which Kotlin compares with a literal past an `Int`'s
    // range as a `Long`.
    if *narrow == Narrow::Checked {
        let most = largest_of(&field.sce_type)
            .filter(|most| *most <= i64::MAX as u64)
            .expect("a length that may not fit its field is held in an integer field");
        let past = format!("{size} > 0x{most:X}");
        let test = match &present {
            Some(there) => format!("{there} && {past}"),
            None => past,
        };
        out.push_str(&format!(
            "if ({test}) {{\n    return {}\n}}\n",
            refusal(plan, RefusalKind::LengthOutOfRange, &named)
        ));
    }
    // Where the content is not, zero, the field gated beside it and not
    // written, or what the value states.
    let number = int_as(&size, &field.sce_type);
    let zero = int_as("0", &field.sce_type);
    let value = match (&present, absent) {
        (None, _) => number,
        (Some(there), Absent::Never | Absent::Zero) => {
            format!("if ({there}) {number} else {zero}")
        }
        (Some(there), Absent::Stated) => format!(
            "if ({there}) {number} else ({} ?: {zero})",
            element(&field.id)
        ),
        (Some(_), Absent::Group) => unreachable!("`unlowered` prints no number a group states"),
    };
    out.push_str(&format!("val _d_{name} = {value}\n"));
    // A value that states the number beside content says it twice, and must
    // say what the content does.
    if *absent == Absent::Stated {
        let there = present
            .as_deref()
            .expect("a number the value states is free: every measure may be absent");
        let stated = element(&field.id);
        out.push_str(&format!(
            "if ({there} && {stated} != null && {stated} != _d_{name}) {{\n    {}\n}}\n",
            disagree()
        ));
    }
    None
}
