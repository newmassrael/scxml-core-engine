// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Swift printer of a codec's encode plan (docs/adr/0081 step 5).
//!
//! A printed codec presents its elements through a protocol, `{Codec}Source`, each element a
//! read-only property of the type the value holds it as, `nil` where a gate leaves it out, and its
//! encode is `{Codec}.encodeSource(_:into:)` over any type that conforms: the host's own type is
//! encoded with no copy into the generated value, as a Rust value is through its source trait
//! (docs/adr/0058 §1). The generated value conforms, and its `encode(into:)` is `encodeSource` of
//! itself. Every decision of the encode is the plan's; this module spells it and decides none.
//!
//! The printer lowers the plan's forms one at a time (docs/adr/0081 §6). A codec whose plan uses a
//! form it does not lower yet keeps the encode of its template, and [`unlowered`] names that form,
//! so no codec has two encodes. The forms printed are a positional frame laid out by byte or
//! written in turn (a fixed number's bytes, a variable-length integer's chain), of numbers (an
//! integer, a flag set presented whole, a value of an imported enum), each always there or decided
//! by a gate, whose flag the encode derives from the presence of what it gates; and byte strings and
//! texts, each held in the runtime's `BoundedBytes` or `BoundedText` over a bound type the codec
//! declares (docs/adr/0065 §6 R1), whose length the encode derives into the field or the flag that
//! holds it, which the value then no longer holds (docs/adr/0081 §3). A flag input the
//! value does not say is a parameter of the encode, as it is of the decode, and a gate on it, which
//! the encode cannot derive, is a check the value is held to before the first byte
//! (`presentIfMismatch`); fields that each say one flag must say the same (`inconsistentSource`).
//! A flag input the value says by the field gated on it alone, or by the body it hands it on to
//! (docs/adr/0065 §6 R4a), is no parameter of the encode: the encode reads it off the value, and so
//! does a parent or a log through the codec's `input{Name}(_:)`; the decode is still handed it.
//! A one-bit flag a parent may set in place of the one the value presents
//! (`EncodePlan::overridable`) is a member of the codec's `Overrides`, which the encode is handed
//! and folds into its carrier.
//!
//! An embedded body of a codec the Swift printer prints is presented as a source of that codec, an
//! associated type of the protocol the generated value binds to the body's own value, and is
//! encoded into bytes of its own once, before the frame is written: its encoded length, where a
//! field holds it, is the count of those bytes (docs/adr/0058 §3, measured once), and a flag bound
//! to an input the body says is read off the body through its codec's `input{Name}(_:)`. The body
//! is handed each input it does not say from the carrier as the encode writes it or from an input
//! of the parent's own, and an input it says that the parent hands it from elsewhere is a check
//! (`presentIfMismatch`).
//!
//! A list or a chain of entries of a codec the Swift printer prints is held in the runtime's
//! `BoundedList` over a count bound the codec declares (docs/adr/0065 §6 R2), each entry a source of
//! the entry codec, an associated type of the protocol as a body's is. Its count, where a field or
//! a flag holds it, is derived from the entries (docs/adr/0081 §3). The walk writes each entry by
//! the entry codec's encode, hands each the inputs a chain supplies from the entry before, and on
//! a chain that ends on its entries' flag sets that flag on every entry but the last through the
//! entry codec's `Overrides` (§6 R7), whatever the entry presents; an input every entry says that
//! the parent hands them from elsewhere is held to it entry by entry before the first byte.
//!
//! A gate group (docs/adr/0065 §6 R4b, R4c) is presented as one property: an enum of its arms,
//! each holding its members by name, or a struct of its one arm, optional where the gates may name
//! no arm. A member is a number, a byte string or a text, a body, a list, or a length or a count
//! the value states on the side where what it measures is absent; the group's type is generic over
//! the source of each body and list it holds, an associated type of the protocol as an element's
//! is. The encode reads each member off the arm the value holds, and derives each gate from which
//! arm that is; a decode makes the arm its gates name.
//!
//! A group the gates on a chain's entries decide (docs/adr/0065 §6 R6) holds the chain in every
//! arm, in the runtime's `SceChain` of the kinds the arm holds an entry of and of none, or, where
//! the one gate on a bare entry holds, in `ScePlaced`, the others and that entry's place; each kind
//! is a `SceKind` the codec declares. The encode reads the chain as the one list of entries every
//! such type holds, walks a bare entry's chain as slots, writing that entry at its place, and asks
//! a chain no group holds for a kind by scanning its entries.
//!
//! A variant every arm of which is a codec the Swift printer prints is presented as the generic
//! enum `{Codec}Arms` over a source of each arm's codec, an associated type of the protocol each,
//! which the generated value binds to the arm codecs' own values. The enum's `tag` is the one table
//! of the arms' tags: the encode derives the tag field or flag from it, and a parent the bits a
//! body's arm fills. The arm is written by its codec's encode, handed what its import binds; an
//! input every arm says, or some arm, is read off the arm the value holds (docs/adr/0065 §6 R4d,
//! R4e), and one the parent hands an arm that says it is held to it before the first byte.

use super::*;
use crate::forge::codec_roles::{ElementId, GateGroup, GateVar, GroupArm, InputSource, InputValue};
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_plan::{
    Absent, Against, Automaton, Bound, CheckSite, Clause, CodecPlan, Derive, Derived, Element,
    EncodePlan, EntryId, Guard, HandedFrom, InputCheck, Law, ListWrite, Measured, Narrow, Number,
    NumberInto, Offset, Presence, PresenceFlag, Value, Write,
};

/// The Swift struct of each codec a codec holds as a body, by the id of the field that holds it.
pub(crate) type HeldTypes = std::collections::HashMap<String, String>;

/// The first form of `plan` the Swift printer does not lower yet, described for a reader; `None`
/// where it prints the codec whole. `imports` says which codec each import is and whether the
/// Swift printer prints it.
pub(crate) fn unlowered(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Option<String> {
    match plan {
        CodecPlan::Positional(plan) => unlowered_positional(plan, imports),
        CodecPlan::Cbor(_) => Some("a CBOR map".to_string()),
        CodecPlan::ContentLine(_) => Some("a component of content lines".to_string()),
    }
}

/// The import `alias` of a codec.
fn import_of<'i>(imports: &'i [ImportContext], alias: &str) -> Option<&'i ImportContext> {
    imports.iter().find(|i| i.alias == alias)
}

/// Whether the codec imported as `alias` says its input `input` in some of its values only.
fn says_partly(imports: &[ImportContext], alias: &str, input: &str) -> bool {
    import_of(imports, alias).is_some_and(|i| i.codec_partly_says_inputs.iter().any(|s| s == input))
}

/// How the encode walks the list or chain `id`, where the program writes one.
fn walk_of<'p>(plan: &'p EncodePlan<'_>, id: &str) -> Option<&'p ListWrite> {
    plan.program.iter().find_map(|write| match write {
        Write::Streamed {
            field,
            list: Some(walk),
            ..
        } if field.id == id => Some(walk),
        _ => None,
    })
}

/// Whether every field of the codec that holds the codec imported as `alias` is a chain that
/// supplies its entries the input `input` from the entry before (`ListWrite::previous`), which the
/// parent then binds nothing to.
fn chain_supplies(plan: &EncodePlan<'_>, alias: &str, input: &str) -> bool {
    let mut holders = plan
        .model
        .fields
        .iter()
        .filter(|field| match presented_value(plan, field) {
            Some(Value::Body { codec } | Value::List { codec, .. }) => codec == alias,
            _ => false,
        })
        .peekable();
    holders.peek().is_some()
        && holders.all(|field| {
            walk_of(plan, &field.id)
                .is_some_and(|walk| walk.previous.iter().any(|(bound, _)| bound == input))
        })
}

/// What the field `field` of the codec of `plan` holds where a value holds it, where the value
/// presents it: as an element of its own, or as a member of the gate group that holds it
/// (docs/adr/0065 §6 R4b, R4c); `None` for a field the encode derives.
fn presented_value<'f>(plan: &EncodePlan<'_>, field: &'f CodecField) -> Option<Value<'f>> {
    let presented =
        plan.elements.iter().any(|e| e.field.id == field.id) || group_of(plan, &field.id).is_some();
    presented.then(|| plan.value(field))
}

/// Whether a value may hold none of the presented field `id` of the codec of `plan`: an element a
/// gate decides or the value states, or a member of a gate group, which an arm the value does not
/// hold leaves out.
fn presented_optional(plan: &EncodePlan<'_>, id: &str) -> bool {
    group_of(plan, id).is_some()
        || plan
            .elements
            .iter()
            .any(|e| e.field.id == id && e.presence.optional())
}

/// The identifier of `entry`, an entry of the chain `chain` of the codec of `plan`, as a `UInt64`
/// (`EncodePlan::entry_ids`): the bits of its carrier, or its field less the flags the chain leaves
/// out, read off the source the chain holds it as.
fn entry_identifier(plan: &EncodePlan<'_>, chain: &str, entry: &str) -> String {
    match plan
        .entry_ids
        .get(chain)
        .expect("a chain whose entries a gate, a bind or a check tells apart declares entry-id")
    {
        EntryId::Flag { carrier, bit, mask } => {
            format!("((chainIdentifier({entry}.{carrier}) >> {bit}) & 0x{mask:X})")
        }
        EntryId::Whole {
            field,
            mask: Some(mask),
        } => format!("(chainIdentifier({entry}.{field}) & 0x{mask:X})"),
        EntryId::Whole { field, mask: None } => format!("chainIdentifier({entry}.{field})"),
    }
}

/// The entry of the kind `key` of a chain of a bare entry (docs/adr/0065 §6 R6a), of the codec
/// `import`, whose Swift struct is `entry`, made by that struct's initializer: its carrier the
/// identifier, and where the codec has a variant, the arm the identifier's tag selects, whose codec
/// presents nothing (`codec_roles::named_entry_is_bare`), made of a frame of zeros, as the C++
/// key's `named()` and Kotlin's `named` make it. The rest of the carrier is what the entry's own
/// encode derives or the walk overrides. `None` where the carrier is no integer or the identifier's
/// tag selects no arm by name.
fn named_entry(import: &ImportContext, entry: &str, key: u64) -> Option<String> {
    let carrier = import.codec_fields.first()?;
    carrier.sce_type.int_bit_width()?;
    let mut arguments = vec![format!("{}: 0x{key:X}", carrier.id)];
    if let Some(variant) = &import.codec_variant {
        if variant.tag_field.as_deref() != Some(carrier.id.as_str()) {
            return None;
        }
        let tag = match variant.tag_flag.as_deref() {
            None => key,
            Some(name) => {
                let flag = carrier.flags.iter().find(|f| f.name == name)?;
                let width = flag.width.max(1);
                let mask = if width >= 64 {
                    u64::MAX
                } else {
                    (1u64 << width) - 1
                };
                key.checked_shr(flag.bit).unwrap_or(0) & mask
            }
        };
        let arm = variant.arms.iter().find(|arm| arm.value == tag)?;
        // The arm's case of the entry's arms enum, whose body's type the case fixes.
        arguments.push(format!(
            "{}: .{}(.init())",
            super::VARIANT_FIELD,
            filters::to_camel_case(arm.body_alias.clone())
        ));
    }
    Some(format!("{entry}({})", arguments.join(", ")))
}

/// Whether the codec of `plan` hands its flag input `name`, which its value says, to a codec it
/// holds, or holds a codec it holds to it, so that the encode reads it off the value.
fn handed_on(plan: &EncodePlan<'_>, name: &str) -> bool {
    plan.handed.values().flatten().any(
        |handed| matches!(&handed.from, Some(HandedFrom::Input { name: from }) if from == name),
    ) || plan
        .input_checks
        .iter()
        .any(|check| matches!(&check.against, Against::Input { name: from } if from == name))
}

/// The place among the gate groups of `plan` of the one that holds the field `id`, where one does.
pub(super) fn group_of(plan: &EncodePlan<'_>, id: &str) -> Option<usize> {
    plan.groups().iter().position(|group| group.contains(id))
}

/// The first gate group of `plan` the Swift printer does not print, described for a reader: one of
/// a bare entry's membership whose entry the printer cannot make of its kind, or whose entries a
/// check holds to an input (docs/adr/0065 §6 R6a), and one that holds a value of a type no number
/// is, or a body or the entries of a codec the Swift printer does not print. A group the printer
/// prints holds numbers, byte strings, texts, bodies and lists, each list held to its law where its
/// entries say inputs it hands them (R8), two lists of one count as one value (R5), and the
/// lengths and counts the value states on the side where what they measure is absent, decided by
/// flags of the codec's carriers, by flag inputs (R4b, R4c), and by the kinds of a chain's
/// entries, each arm holding the chain in a type of the kinds it says (R6). `imports` says which
/// codec each import is and whether the Swift printer prints it.
fn group_unlowered(plan: &EncodePlan<'_>, imports: &[ImportContext]) -> Option<String> {
    for group in plan.groups() {
        let label = super::super::rust_source::gates_label(group);
        if let Some(chain) = group.chain().filter(|_| group.member_is_bare()) {
            // The walk writes the bare entry at its place, made of its kind.
            let Value::List { codec, .. } = plan.value(field_named(plan, chain)) else {
                unreachable!("the chain whose memberships are gates is a list")
            };
            let named = group
                .members()
                .next()
                .and_then(|(_, key)| named_entry(import_of(imports, codec)?, "Entry", key));
            if named.is_none() {
                return Some(format!(
                    "the gate group `{label}`, whose bare entry the Swift printer cannot make of \
                     its kind"
                ));
            }
            // A check of the entries a walk of slots writes would see the others alone, the named
            // entry left out, as the C++ and Kotlin printers refuse them.
            let checked = plan
                .input_checks
                .iter()
                .any(|check| matches!(&check.site, CheckSite::List { list } if list.id == chain));
            if checked {
                return Some(format!(
                    "the gate group `{label}`, whose chain of a bare entry's place holds entries \
                     a check holds to an input"
                ));
            }
        }
        // Two lists of one count are held as one `Aligned` (docs/adr/0065 §6 R5) of two
        // collections, each a list within its bound and, where its entries say inputs it hands
        // them, held to its law (R8): a `Lawful` reads as its list, so the pair holds it as any.
        for id in group.fields() {
            let what = match plan.value(field_named(plan, id)) {
                Value::Scalar(t) if is_number(t) => None,
                Value::Scalar(t) => Some(format!("a `{}` value", t.as_attr())),
                Value::Bytes(_) | Value::Text(_) => None,
                // A member body or list is written by its own codec's printed encode, as an
                // element is.
                Value::Body { codec } => match import_of(imports, codec) {
                    Some(import) if import.swift_printed => None,
                    _ => Some(format!(
                        "an embedded body of `{codec}`, which the Swift printer does not print,"
                    )),
                },
                // A list held to its law in the arm that holds it (docs/adr/0065 §6 R8) is held
                // so there as an element is.
                Value::List { codec, .. } => match import_of(imports, codec) {
                    Some(import) if import.swift_printed => None,
                    _ => Some(format!(
                        "a list of `{codec}`, which the Swift printer does not print,"
                    )),
                },
            };
            if let Some(what) = what {
                return Some(format!(
                    "field '{id}': {what} the gate group `{label}` holds"
                ));
            }
        }
    }
    None
}

/// The generic parameters of the type of the gate group `group` of `plan`, each with the protocol
/// it is bound to: one for each body the group holds, a source of the body's codec, and one for
/// each list, a source of its entries' codec, named as the associated type of the codec's
/// protocol that the value presents it by ([`body_source_type`], [`entry_source_type`]), in the
/// order the arms first name the members. A group of numbers, byte strings and texts has none.
/// `held` is the Swift struct of each body and list's entries by the field that holds it.
fn group_generics(
    plan: &EncodePlan<'_>,
    group: &GateGroup,
    held: &HeldTypes,
) -> Vec<(String, String)> {
    group
        .fields()
        .into_iter()
        .filter_map(|id| {
            let name = match plan.value(field_named(plan, id)) {
                Value::Body { .. } => body_source_type(id),
                Value::List { .. } => entry_source_type(id),
                _ => return None,
            };
            let child = held
                .get(id)
                .expect("a body or a list a group holds holds a codec of the module");
            Some((name, format!("{child}Source")))
        })
        .collect()
}

/// The type a value of `plan` holds the gate group `group` in, with the source of each body and
/// list it holds named by the associated type the codec's protocol presents it by (which the
/// generated value binds to the codec's own value), optional where the value may hold no arm.
fn group_holds(plan: &EncodePlan<'_>, group: &GateGroup, held: &HeldTypes) -> String {
    let struct_name = filters::to_pascal_case(plan.model.name.clone());
    let ty = group_type_name(&struct_name, group);
    let arguments: Vec<String> = group_generics(plan, group, held)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    let ty = if arguments.is_empty() {
        ty
    } else {
        format!("{ty}<{}>", arguments.join(", "))
    };
    if group.absent.is_some() {
        format!("{ty}?")
    } else {
        ty
    }
}

/// The property a printed value holds the gate group `group` in, and the label a member of a
/// decode and an initializer names it by: the group's accessor, as every printer names it.
pub(super) fn group_name(group: &GateGroup) -> String {
    super::super::rust_source::group_accessor(group)
}

/// The type a printed value of the codec named `struct_name` holds the gate group `group` as where
/// it holds an arm of it: an enum of its arms, or a struct of its one arm, named as every printer
/// names it.
pub(super) fn group_type_name(struct_name: &str, group: &GateGroup) -> String {
    super::super::rust_source::group_type(struct_name, group)
}

/// The case of the enum of `group`'s arms that holds `arm`: the arm's name, its first letter in
/// lower case.
pub(super) fn group_case(group: &GateGroup, arm: &GroupArm) -> String {
    let name = super::super::rust_source::group_arm_name(group, arm);
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => unreachable!("an arm of a group has a name"),
    }
}

/// The field `id` of the codec of `plan`.
fn field_named<'m>(plan: &EncodePlan<'m>, id: &str) -> &'m CodecField {
    plan.model
        .fields
        .iter()
        .find(|f| f.id == id)
        .expect("a group's member is a field of the codec")
}

/// The Swift type a member `field` of a gate group of `plan` is held as within its arm: its number,
/// its byte string or text in the bound its codec declares (docs/adr/0065 §6 R1), the source of
/// its body, or its list of sources of its entries within the count bound its codec declares
/// (R2). A source is named by the generic parameter of the group's type ([`group_generics`]),
/// reached through `of`: nothing in the type's own declaration, `Source.` in an encode over a
/// source of the codec, whose associated type of that name it is.
fn member_type(l: &LangCtx, plan: &EncodePlan<'_>, field: &CodecField, of: &str) -> String {
    let struct_name = filters::to_pascal_case(plan.model.name.clone());
    match plan.value(field) {
        Value::Body { .. } => format!("{of}{}", body_source_type(&field.id)),
        Value::List { law, .. } => list_type(
            &struct_name,
            &field.id,
            &format!("{of}{}", entry_source_type(&field.id)),
            &law,
        ),
        value => bounded_value_type(&struct_name, &field.id, &value)
            .unwrap_or_else(|| l.type_name(&field.sce_type).into_owned()),
    }
}

/// A list a printed value holds, as the template writes it.
pub(super) struct Listed {
    /// Its type.
    pub(super) ty: String,
    /// The expression a decode makes it by of the array of entries it read.
    pub(super) made: String,
    /// Its value before a frame is read.
    pub(super) default: String,
}

/// The list a printed value of the codec named `struct_name` holds the field `field` of `plan` in,
/// an element or a member of a gate group, each entry an `entry` (docs/adr/0065 §6 R2, R8): its
/// type, the expression a decode makes it by of the array `items` it read, through the door of
/// its bound and, where its entries say inputs it hands them, of its law, from the start the frame
/// gave, and its value before a frame is read; `None` for a field that holds no list, or that the
/// value does not present.
pub(super) fn listed_field_type(
    struct_name: &str,
    plan: &EncodePlan<'_>,
    field: &CodecField,
    entry: &str,
    items: &str,
) -> Option<Listed> {
    let Value::List { law, .. } = presented_value(plan, field)? else {
        return None;
    };
    let ty = list_type(struct_name, &field.id, entry, &law);
    let bounded = format!(
        "try {}(decoding: {items})",
        bounded_list_type(struct_name, &field.id, entry)
    );
    let made = if law.is_none() {
        bounded
    } else {
        format!(
            "try {ty}(decoding: {bounded}, start: {})",
            law_start_read(plan, field)
        )
    };
    Some(Listed {
        ty,
        made,
        default: list_default(entry, &law),
    })
}

/// The bounded type a printed value of the codec named `struct_name` holds the field `field` of
/// `plan` in, an element or a member of a gate group (docs/adr/0065 §6 R1); `None` for a field
/// that holds no byte string or text, or that the value does not present.
pub(super) fn bounded_field_type(
    struct_name: &str,
    plan: &EncodePlan<'_>,
    field: &CodecField,
) -> Option<String> {
    if let Some(element) = plan.elements.iter().find(|e| e.field.id == field.id) {
        return bounded_type(struct_name, element);
    }
    group_of(plan, &field.id)?;
    bounded_value_type(struct_name, &field.id, &plan.value(field))
}

/// The `Bool` the gate `gate` of `group` holds for `held`, the group as a source presents it
/// (docs/adr/0065 §6 R4c): what the arm `held` holds gives it, read off the arm's place among the
/// arms (`arm`) where the arms do not all give it the same, and what the gates are where `held` is
/// `nil`.
fn gate_value(group: &GateGroup, gate: usize, held: &str) -> String {
    let set: Vec<String> = (0..group.arms.len())
        .filter(|&index| group.arms[index].values[gate])
        .map(|index| index.to_string())
        .collect();
    // What the gate is while a value holds the arm whose place is `arm`.
    let of_arm = |arm: &str| -> String {
        if set.is_empty() {
            "false".to_string()
        } else if set.len() == group.arms.len() {
            "true".to_string()
        } else {
            format!("[{}].contains({arm})", set.join(", "))
        }
    };
    let Some(absent) = &group.absent else {
        // A value always holds an arm of it.
        return of_arm(&format!("{held}.arm"));
    };
    let none = absent[gate];
    let some = of_arm("$0.arm");
    if some == "true" || some == "false" {
        if some == none.to_string() {
            some
        } else {
            format!("({held} != nil ? {some} : {none})")
        }
    } else {
        format!("({held}.map {{ {some} }} ?? {none})")
    }
}

/// The test, on the locals a decode reads the frame into, that the gates of `group` hold the
/// values of `arm`: a flag input by its parameter, a one-bit flag of the codec's own by its
/// carrier, and a chain's membership by the local the decode binds once it has read the chain
/// (docs/adr/0065 §6 R6), as the template's gates read it.
fn decode_arm_test(plan: &EncodePlan<'_>, group: &GateGroup, arm: &GroupArm) -> String {
    group
        .gates
        .iter()
        .zip(&arm.values)
        .map(|(gate, value)| {
            let cmp = if *value { "!=" } else { "==" };
            match gate {
                GateVar::Input(input) => format!("({input} & 0x01) {cmp} 0"),
                GateVar::Flag { carrier, flag } => {
                    let bit = field_named(plan, carrier)
                        .flags
                        .iter()
                        .find(|f| f.name == *flag)
                        .map(|f| f.bit)
                        .expect("a gate's flag is a flag of the codec's carrier");
                    format!("(({} >> {bit}) & 1) {cmp} 0", super::decoded(carrier))
                }
                GateVar::Member { chain, key, .. } => {
                    let has = super::super::chain_has_local(
                        chain,
                        *key,
                        crate::generator::Language::Swift,
                    );
                    if *value {
                        has
                    } else {
                        format!("!{has}")
                    }
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" && ")
}

/// The local a decode makes the gate group `group` in, once the frame is read, from the locals of
/// its members: the group's name and `_group`, beside the locals a decode reads each field into
/// under the field's own name.
fn group_local(group: &GateGroup) -> String {
    format!("{}_group", group_name(group))
}

/// One thing a printed value presents: an element, or a gate group by its place among the groups.
pub(super) enum Presented<'p, 'm> {
    Element(&'p Element<'m>),
    Group(usize),
}

/// What a printed value of `plan` presents, in the order the document declares its fields: each
/// element, and each gate group where its first member stands. The protocol, the initializer and a
/// case all read the value in this order.
pub(super) fn presented_order<'p, 'm>(plan: &'p EncodePlan<'m>) -> Vec<Presented<'p, 'm>> {
    let mut out = Vec::new();
    let mut groups = std::collections::BTreeSet::new();
    for field in &plan.model.fields {
        if let Some(element) = plan.elements.iter().find(|e| e.field.id == field.id) {
            out.push(Presented::Element(element));
        } else if let Some(index) = group_of(plan, &field.id) {
            if groups.insert(index) {
                out.push(Presented::Group(index));
            }
        }
    }
    out
}

/// The local the encode holds the member `id` of a gate group in: what the arm the value holds
/// holds of it, `nil` where it holds none. A field is read through the source and never bare, so
/// the local hides no field.
fn member_local(id: &str) -> String {
    format!("{id}_member")
}

/// The local the encode holds the place of the bare entry of the chain `id` in, where the arm the
/// value holds holds it (docs/adr/0065 §6 R6a), `nil` where it holds none.
fn place_local(id: &str) -> String {
    format!("{id}_place")
}

/// What the encode reads the presented field `id` of `plan` off: the source's property of an
/// element, or the local of a group's member, optional as the field's gate makes it.
fn presented_read(plan: &EncodePlan<'_>, id: &str) -> String {
    if group_of(plan, id).is_some() {
        member_local(id)
    } else {
        format!("{SOURCE}.{id}")
    }
}

/// The source `encodeSource` is handed, through which a type of the encode names an associated
/// type of the codec's protocol.
const OF_SOURCE: &str = "Source.";

/// One member of an arm of a gate group as a value holds it: a field, or two lists of one count
/// held as one value (docs/adr/0065 §6 R5).
pub(super) enum ArmMember<'g> {
    One(&'g str),
    Aligned { first: &'g str, second: &'g str },
}

impl ArmMember<'_> {
    /// The name the arm holds the member by: the field's, or the two lists' joined, as every
    /// printer names them (`rust_source::aligned_name`).
    pub(super) fn name(&self) -> String {
        match self {
            ArmMember::One(id) => (*id).to_string(),
            ArmMember::Aligned { first, second } => {
                super::super::rust_source::aligned_name(first, second)
            }
        }
    }
}

/// The members `arm` of `group` holds, in the order of its fields: each field, less the second of
/// two lists of one count, which the first holds with it (docs/adr/0065 §6 R5).
pub(super) fn arm_members<'g>(group: &'g GateGroup, arm: &'g GroupArm) -> Vec<ArmMember<'g>> {
    arm.fields
        .iter()
        .filter(|id| !group.is_second_in(arm, id))
        .map(|id| match group.second_of(arm, id) {
            Some(second) => ArmMember::Aligned { first: id, second },
            None => ArmMember::One(id),
        })
        .collect()
}

/// The Swift type the member `member` of the arm `arm` of the gate group `group` of `plan` is held
/// as, its sources reached through `of` ([`member_type`]): two lists of one count as the runtime's
/// `Aligned` of the two (docs/adr/0065 §6 R5), and the chain whose memberships are the group's
/// gates in the type of the kinds the arm says of it ([`chain_arm_type`], R6).
fn arm_member_type(
    l: &LangCtx,
    plan: &EncodePlan<'_>,
    group: &GateGroup,
    arm: &GroupArm,
    member: &ArmMember<'_>,
    of: &str,
) -> String {
    match member {
        ArmMember::One(id) if is_group_chain(group, id) => chain_arm_type(
            &filters::to_pascal_case(plan.model.name.clone()),
            group,
            arm,
            field_named(plan, id),
            &format!("{of}{}", entry_source_type(id)),
        ),
        ArmMember::One(id) => member_type(l, plan, field_named(plan, id), of),
        ArmMember::Aligned { first, second } => format!(
            "Aligned<{}, {}>",
            member_type(l, plan, field_named(plan, first), of),
            member_type(l, plan, field_named(plan, second), of)
        ),
    }
}

/// The statements that read each member of the gate group `group` off the arm a source of the
/// codec of `plan` holds into the member's local ([`member_local`]), `nil` where that arm does not
/// hold it: a switch over the arms of a sum, each arm's members bound by their names, or the one
/// arm's members where the value holds it. A body or a list is kept as the source presents it,
/// of the source's associated type, so a host's type is written with no copy.
fn group_member_lines(l: &LangCtx, plan: &EncodePlan<'_>, group: &GateGroup) -> Vec<String> {
    let held = format!("{SOURCE}.{}", group_name(group));
    let mut lines: Vec<String> = Vec::new();
    for id in group.fields() {
        if is_group_chain(group, id) {
            // The chain every arm holds, in a type of the kinds the arm says (docs/adr/0065 §6
            // R6), read as the one list of entry sources every such type holds; where a bare
            // entry's place is held, the others and the place.
            lines.push(format!(
                "var {}: [{OF_SOURCE}{}]? = nil",
                member_local(id),
                entry_source_type(id)
            ));
            if group.member_is_bare() {
                lines.push(format!("var {}: Int? = nil", place_local(id)));
            }
            continue;
        }
        lines.push(format!(
            "var {}: {}? = nil",
            member_local(id),
            member_type(l, plan, field_named(plan, id), OF_SOURCE)
        ));
    }
    // The members of `arm` set from `from`, each the expression of one by its name: two lists of
    // one count each read out of the pair (docs/adr/0065 §6 R5), and the chain whose memberships
    // are the gates as its entries, or the others and the place of a bare entry (R6).
    let assign = |arm: &GroupArm, from: &dyn Fn(&str) -> String| -> String {
        arm_members(group, arm)
            .iter()
            .flat_map(|member| match member {
                ArmMember::One(id) if is_group_chain(group, id) => {
                    let chain = from(id);
                    if holds_place(group, arm) {
                        vec![
                            format!("{} = {chain}.rest", member_local(id)),
                            format!("{} = {chain}.place", place_local(id)),
                        ]
                    } else if super::super::rust_source::chain_optional_in(
                        group,
                        field_named(plan, id),
                        arm,
                    ) {
                        vec![format!("{} = {chain}?.entries", member_local(id))]
                    } else {
                        vec![format!("{} = {chain}.entries", member_local(id))]
                    }
                }
                ArmMember::One(id) => vec![format!("{} = {}", member_local(id), from(id))],
                ArmMember::Aligned { first, second } => {
                    let pair = from(&member.name());
                    vec![
                        format!("{} = {pair}.first", member_local(first)),
                        format!("{} = {pair}.second", member_local(second)),
                    ]
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    if group.is_sum() {
        let optional = if group.absent.is_some() { "?" } else { "" };
        lines.push(format!("switch {held} {{"));
        for arm in &group.arms {
            let case = group_case(group, arm);
            let set = assign(arm, &|name: &str| name.to_string());
            let line = if arm.fields.is_empty() {
                format!("case .{case}{optional}: break")
            } else {
                let bound: Vec<String> = arm_members(group, arm)
                    .iter()
                    .map(|member| format!("let {}", member.name()))
                    .collect();
                format!("case .{case}({}){optional}: {set}", bound.join(", "))
            };
            lines.push(line);
        }
        if group.absent.is_some() {
            lines.push("case nil: break".to_string());
        }
        lines.push("}".to_string());
    } else {
        let arm = &group.arms[0];
        let set = assign(arm, &|id: &str| format!("{HELD}.{id}"));
        lines.push(if group.absent.is_some() {
            format!("if let {HELD} = {held} {{ {set} }}")
        } else {
            format!("do {{ let {HELD} = {held}; {set} }}")
        });
    }
    lines
}

/// What the template writes for the gate group `group` of the codec of `plan` (docs/adr/0065 §6
/// R4b, R4c): its type, an enum of its arms each holding its members by name (with the place of
/// the arm among them, `arm`, which the encode reads the gates off) or a struct of its one arm; the
/// value it starts at, `nil` where it may hold no arm and otherwise the arm of a frame of zeros,
/// each member at its own start; and the statements that make it, once the frame is read, from
/// the locals the decode read each member into, the arm the gates the frame holds name. A group
/// that holds a body or a list is generic over the source of each ([`group_generics`]), as the
/// variant is over its arms, so a host's type is presented with no copy; it is `Equatable` and
/// `Sendable` where those sources are. `held` is the Swift struct of each body and list's entries
/// by the field that holds it.
fn group_context(
    l: &LangCtx,
    plan: &EncodePlan<'_>,
    group: &GateGroup,
    held: &HeldTypes,
) -> serde_json::Value {
    let struct_name = filters::to_pascal_case(plan.model.name.clone());
    let ty = group_type_name(&struct_name, group);
    let generics: Vec<serde_json::Value> = group_generics(plan, group, held)
        .into_iter()
        .map(|(name, protocol)| serde_json::json!({ "name": name, "protocol": protocol }))
        .collect();
    // The arm `arm` holding the members `value` gives each field of it, by its name, and two lists
    // of one count as one value of what `value` gives each, made by `pair` (docs/adr/0065 §6 R5).
    let make = |arm: &GroupArm,
                value: &dyn Fn(&GroupArm, &CodecField) -> String,
                pair: &dyn Fn(String, String) -> String|
     -> String {
        let members: Vec<String> = arm_members(group, arm)
            .iter()
            .map(|member| {
                let made = match member {
                    ArmMember::One(id) => value(arm, field_named(plan, id)),
                    ArmMember::Aligned { first, second } => pair(
                        value(arm, field_named(plan, first)),
                        value(arm, field_named(plan, second)),
                    ),
                };
                format!("{}: {made}", member.name())
            })
            .collect();
        match (group.is_sum(), members.is_empty()) {
            (true, true) => format!(".{}", group_case(group, arm)),
            (true, false) => format!(".{}({})", group_case(group, arm), members.join(", ")),
            (false, _) => format!("{ty}({})", members.join(", ")),
        }
    };
    // The local a decode makes the chain of an arm that may hold none in, before it makes the arm
    // (docs/adr/0065 §6 R6b).
    let chain_local = |id: &str| format!("{id}_chain");
    // Each gated member of an arm the gates name was read where the gate holds it, which they do;
    // a length or a count the value states on the side where what it measures is absent is read
    // always. The chain whose memberships are the gates is made of the chain read, through the door
    // of the type of the kinds the arm says, which the kinds the arm was selected by hold: the
    // others and the place of the first entry of a bare entry's kind, the chain whole, or, where
    // the arm may hold none, the local made before the arm.
    let read = |arm: &GroupArm, f: &CodecField| {
        let decoded = if f.present_if.is_some() {
            format!("{}!", super::decoded(&f.id))
        } else {
            super::decoded(&f.id)
        };
        if !is_group_chain(group, &f.id) {
            decoded
        } else if holds_place(group, arm) {
            format!("try ScePlaced(decoding: {decoded})")
        } else if super::super::rust_source::chain_optional_in(group, f, arm) {
            chain_local(&f.id)
        } else {
            format!("try SceChain(decoding: {decoded})")
        }
    };
    // Two lists read by one count are of one count, which the decode's door holds them to.
    let read_pair =
        |first: String, second: String| format!("try Aligned(decoding: {first}, {second})");
    let local = group_local(group);
    let holds = group_holds(plan, group, held);
    let mut decode = vec![format!("let {local}: {holds}")];
    let last = group.arms.len() - 1;
    for (index, arm) in group.arms.iter().enumerate() {
        let opening = match (index, group.absent.is_some() || index < last) {
            (0, true) => format!("if {} {{", decode_arm_test(plan, group, arm)),
            (_, true) => format!("}} else if {} {{", decode_arm_test(plan, group, arm)),
            (0, false) => "do {".to_string(),
            (_, false) => "} else {".to_string(),
        };
        decode.push(opening);
        // The chain of an arm that may hold none: made where the frame held one, as the arm says.
        if let Some(chain) = group.chain().map(|id| field_named(plan, id)) {
            if !holds_place(group, arm)
                && super::super::rust_source::chain_optional_in(group, chain, arm)
            {
                let id = chain.id.as_str();
                let ty = arm_member_type(l, plan, group, arm, &ArmMember::One(id), "");
                let made = chain_local(id);
                decode.push(format!("    var {made}: {ty} = nil"));
                decode.push(format!(
                    "    if let {id}_read = {} {{ {made} = try {}(decoding: {id}_read) }}",
                    super::decoded(id),
                    ty.trim_end_matches('?')
                ));
            }
        }
        decode.push(format!("    {local} = {}", make(arm, &read, &read_pair)));
    }
    if group.absent.is_some() {
        decode.push("} else {".to_string());
        decode.push(format!("    {local} = nil"));
    }
    decode.push("}".to_string());
    // A frame of zeros holds the arm whose gates are all clear, each member at its start: an enum's
    // first value, a carrier's flags' declared values, no bytes, a body's own value of a frame of
    // zeros, the fewest entries a list's bound admits, from the start they say where a law holds
    // them (docs/adr/0065 §6 R8), and a chain of no kind, none where it is gated, else the fewest
    // entries its bound admits, held to lack every kind the gates name (R6b).
    let child = |f: &CodecField| {
        held.get(&f.id)
            .expect("a body or a list a group holds holds a codec of the module")
            .clone()
    };
    let start = |arm: &GroupArm, f: &CodecField| match (&plan.value(f), &f.sce_type) {
        (Value::List { law, .. }, _) if is_group_chain(group, &f.id) => {
            if super::super::rust_source::chain_optional_in(group, f, arm) {
                "nil".to_string()
            } else {
                format!(".init(defaultOf: {})", list_default(&child(f), law))
            }
        }
        (Value::Bytes(_) | Value::Text(_), _) => ".fewest".to_string(),
        (Value::Body { .. }, _) => format!("{}()", child(f)),
        (Value::List { law, .. }, _) => list_default(&child(f), law),
        (_, SceType::Enum(r)) => l.enum_default_expr(&r.alias),
        _ => match flag_default_carrier_value(f) {
            0 => "0".to_string(),
            bits => format!("0x{bits:X}"),
        },
    };
    let default = if group.absent.is_some() {
        "nil".to_string()
    } else {
        let zeros = group
            .arms
            .iter()
            .find(|arm| arm.values.iter().all(|value| !value))
            .expect("a group a value always holds an arm of has an arm for every gate clear");
        let pair = |first: String, second: String| format!(".init(defaultOf: {first}, {second})");
        make(zeros, &start, &pair)
    };
    let arms: Vec<serde_json::Value> = group
        .arms
        .iter()
        .enumerate()
        .map(|(index, arm)| {
            let when: Vec<String> = group
                .gates
                .iter()
                .zip(&arm.values)
                .map(|(gate, value)| format!("{gate} {}", if *value { "set" } else { "clear" }))
                .collect();
            serde_json::json!({
                "case": group_case(group, arm),
                "index": index,
                "when": when.join(", "),
                "members": arm_members(group, arm)
                    .iter()
                    .map(|member| serde_json::json!({
                        "name": member.name(),
                        "type": arm_member_type(l, plan, group, arm, member, ""),
                    }))
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    serde_json::json!({
        "name": group_name(group),
        "type": ty,
        "generics": generics,
        "optional": group.absent.is_some(),
        "holds": holds,
        "label": super::super::rust_source::gates_label(group),
        "sum": group.is_sum(),
        "arms": arms,
        "default": default,
        "decode": decode,
    })
}

/// [`unlowered`] of a positional codec's plan.
pub(crate) fn unlowered_positional(
    plan: &EncodePlan<'_>,
    imports: &[ImportContext],
) -> Option<String> {
    // An arm is written by its own codec's printed encode, so the Swift printer prints the codec of
    // every arm first.
    if let Some(arm) = plan
        .arms
        .iter()
        .find(|arm| !import_of(imports, arm.codec).is_some_and(|import| import.swift_printed))
    {
        return Some(format!(
            "the arm `{}` of the variant, which the Swift printer does not print",
            arm.codec
        ));
    }
    if let Some(what) = group_unlowered(plan, imports) {
        return Some(what);
    }
    for element in &plan.elements {
        let id = &element.field.id;
        let what = match &element.value {
            Value::Scalar(t) if is_number(t) => None,
            Value::Scalar(t) => Some(format!("a `{}` value", t.as_attr())),
            Value::Bytes(_) | Value::Text(_) => None,
            // A body is written by its own codec's printed encode, so the Swift printer prints the
            // codec it holds first.
            Value::Body { codec } => match import_of(imports, codec) {
                Some(import) if import.swift_printed => None,
                _ => Some(format!(
                    "an embedded body of `{codec}`, which the Swift printer does not print"
                )),
            },
            // A list's entries are written by their own codec's printed encode, so the Swift
            // printer prints the codec they are first; a list whose entries say the inputs it
            // hands them is held to its law (docs/adr/0065 §6 R8).
            Value::List { codec, .. } => match import_of(imports, codec) {
                Some(import) if import.swift_printed => None,
                _ => Some(format!(
                    "a list of `{codec}`, which the Swift printer does not print"
                )),
            },
        };
        // A number the value states where what it measures is absent (`Presence::Stated`) is
        // presented as an optional and derived where what it measures is there (docs/adr/0058 §1).
        if let Some(what) = what {
            return Some(format!("field '{id}': {what}"));
        }
    }
    // A body or an entry is handed each input it does not say from the parent's carrier or its
    // own input, and an entry of a chain, besides, an input the chain supplies from the entry
    // before (`ListWrite::previous`).
    if let Some((alias, handed)) = plan.handed.iter().find_map(|(alias, handed)| {
        Some((
            alias,
            handed
                .iter()
                .find(|h| h.from.is_none() && !chain_supplies(plan, alias, &h.input))?,
        ))
    }) {
        return Some(format!(
            "the input `{}` of `{alias}`, which no declaration binds",
            handed.input
        ));
    }
    // A presence flag is read off the fields it gates that say it: the first, or every one where
    // they may disagree, each an element or a member of the group that holds it.
    let presented = |f: &CodecField| {
        plan.elements.iter().any(|e| e.field.id == f.id) || group_of(plan, &f.id).is_some()
    };
    let readable = |presence: &PresenceFlag<'_>| {
        let (first, rest) = match presence.of.split_first() {
            Some(split) => split,
            None => return false,
        };
        presented(first.0) && (presence.together || rest.iter().all(|&(f, _)| presented(f)))
    };
    // The flag a chain's entries end on is set by the walk on every entry but the last, through
    // the entry codec's `Overrides` (docs/adr/0065 §6 R7).
    let continued = |derived: &&Derived<'_>| match (&derived.element, &derived.by) {
        (ElementId::Flag { field, .. }, Derive::ChainContinuation) => {
            walk_of(plan, field).is_some_and(|walk| walk.ends.is_some())
        }
        _ => false,
    };
    // The tag of the arm the value holds, and the bits of a carrier the arm of an embedded body's
    // variant fills, are read off the arm through the arms enum's `tag`.
    let tagged = |derived: &&Derived<'_>| {
        matches!(
            (&derived.element, &derived.by),
            (
                ElementId::Field(_) | ElementId::Flag { .. },
                Derive::VariantTag
            ) | (ElementId::Flag { .. }, Derive::ParentTag(_))
        )
    };
    // A flag of a disjunction that decides a group is what the arm the value holds gives it
    // (docs/adr/0065 §6 R4c).
    let of_an_arm = |derived: &&Derived<'_>| matches!(derived.by, Derive::ArmFlag { .. });
    if let Some(derived) = plan.derivations.iter().find(|derived| {
        let presence = presence_flag(&derived.element, &derived.by)
            .is_some_and(|(_, _, presence)| readable(presence));
        !presence
            && printed_number(&derived.element, &derived.by).is_none()
            && !continued(derived)
            && !tagged(derived)
            && !of_an_arm(derived)
    }) {
        return Some(format!("`{}`, which the encode derives", derived.element));
    }
    // A body that says an input in some of its values only says nothing in the others.
    if let Some(derived) = plan.derivations.iter().find(|derived| {
        printed_number(&derived.element, &derived.by).is_some_and(|number| {
            number.measures.iter().any(|measure| match &measure.of {
                Measured::Says { codec, input, .. } => says_partly(imports, codec, input),
                Measured::ArmSays { arms } => arms
                    .iter()
                    .any(|arm| says_partly(imports, &arm.codec, &arm.input)),
                _ => false,
            })
        })
    }) {
        return Some(format!(
            "`{}`, bound to an input its body says in some of its values only",
            derived.element
        ));
    }
    // An input the value says is read off the field gated on it alone, off the arm the gate group
    // it decides holds, off the body always there that it hands it on to and that says it in every
    // value, off the arm of the variant the value holds, or off the start of the list it hands it
    // on to, held to its law (docs/adr/0065 §6 R8), as an element of its own or in the arm of the
    // gate group that holds it, where an arm that does not hold it says nothing (§6 R4e). One some
    // arms say only is said in some values only, and no encode reads it (docs/adr/0065 §6 R4e):
    // only a parent asks for it.
    if let Some(said) = plan
        .said
        .iter()
        .find(|said| match &plan.roles.inputs[&said.input.name] {
            InputSource::Gate { .. }
            | InputSource::Group
            | InputSource::Forward { partly: false, .. }
            | InputSource::EveryArm { .. }
            | InputSource::List { .. } => false,
            InputSource::SomeArms { .. } => said.read || handed_on(plan, &said.input.name),
            InputSource::Forward { partly: true, .. } => true,
        })
    {
        return Some(format!(
            "the flag input `{}`, which the value says by what it holds",
            said.input.name
        ));
    }
    if let Some(refusal) = plan
        .totality
        .refusals()
        .iter()
        .find(|refusal| !writes_refusal(plan, refusal.kind, &refusal.element))
    {
        return Some(format!(
            "an encode that refuses a value at `{}` ({})",
            refusal.element, refusal.kind
        ));
    }
    plan.program.iter().find_map(|write| match write {
        Write::Whole(_) | Write::Byte { .. } => None,
        // A number or a byte string or text in turn, always there or where the value holds it, a
        // number the encode derives whole where its gate holds, a gate on a chain's kinds among
        // them (docs/adr/0065 §6 R6), and a body where the value holds it: the bytes of its
        // carrier, its chain, its bytes, or the body's own frame.
        Write::Streamed {
            field, list: None, ..
        } if matches!(
            field.bit_size,
            BitSize::Fixed { .. }
                | BitSize::Vle { .. }
                | BitSize::Tail
                | BitSize::LengthRef
                | BitSize::Embed
        ) =>
        {
            None
        }
        // A list or a chain where the value holds it, each entry written in turn by its codec's
        // printed encode, handed what the entry before supplies and, on a chain that ends on its
        // entries' flag, the flag set on every entry but the last. A list held to its law is
        // walked as any list: its type holds every entry to the law where it is made
        // (docs/adr/0065 §6 R8). A chain of a bare entry's place is walked as slots (R6a).
        Write::Streamed {
            list: Some(_),
            guard: Guard::Always | Guard::Present,
            ..
        } => None,
        // The bytes or text after a frame laid out by byte, never gated (a gate makes a frame one
        // written in turn).
        Write::Tail { .. } => None,
        Write::Streamed { field, .. } => {
            Some(format!("field '{}': a field written in turn", field.id))
        }
        // The arm the value holds, written by its codec's printed encode.
        Write::Variant => None,
    })
}

/// The number `element` is, where `by` derives it as one the printer writes: a number of the byte
/// strings and texts it measures, of the entries of the lists it counts, of the inputs the bodies
/// it measures say, of the start of the lists held to a law it is bound to (docs/adr/0065 §6 R8),
/// or of the bytes the bodies it measures encode to, into a field of its own
/// (zero where a gate leaves every one of them out, beside which the field is gated too, what the
/// arm of the gate group that holds the field states there, docs/adr/0065 §6 R4b, or what the value
/// states there, docs/adr/0058 §1) or, but for an encoded length, into a flag of a carrier.
fn printed_number<'d, 'm>(element: &'d ElementId, by: &'d Derive<'m>) -> Option<&'d Number<'m>> {
    let Derive::Number(number) = by else {
        return None;
    };
    let encoded = |measured: &Measured<'_>| matches!(measured, Measured::Encoded { .. });
    let measures = number.measures.iter().all(|measure| {
        matches!(
            measure.of,
            Measured::Bytes { .. }
                | Measured::Entries { .. }
                | Measured::Says { .. }
                | Measured::ArmSays { .. }
                | Measured::Start { .. }
                | Measured::Encoded { .. }
        )
    });
    // An encoded length is counted once the bodies are encoded, after every carrier is folded, so
    // it goes into a field and measures nothing else.
    let counted = !number.measures.iter().any(|m| encoded(&m.of))
        || (matches!(element, ElementId::Field(_))
            && number.measures.iter().all(|m| encoded(&m.of)));
    let into = matches!(
        (element, &number.into),
        (ElementId::Field(_), NumberInto::Field { .. })
            | (ElementId::Flag { .. }, NumberInto::Flag)
    );
    (measures && counted && into).then_some(number)
}

/// Whether `number` is the encoded length of bodies, counted once they are encoded.
fn is_encoded_length(number: &Number<'_>) -> bool {
    number
        .measures
        .iter()
        .any(|measure| matches!(measure.of, Measured::Encoded { .. }))
}

/// The presence flag `element` is, where `by` derives it as one: a flag of a carrier set exactly
/// where what it gates is present.
fn presence_flag<'d, 'm>(
    element: &'d ElementId,
    by: &'d Derive<'m>,
) -> Option<(&'d str, &'d str, &'d PresenceFlag<'m>)> {
    match (element, by) {
        (ElementId::Flag { field, flag }, Derive::Presence(presence)) => {
            Some((field.as_str(), flag.as_str(), presence))
        }
        _ => None,
    }
}

/// Whether the printer writes the refusal of `kind` at `element`: the presence of a field held to a
/// gate the encode cannot derive, or an input a body or the entries of a list say held to what the
/// parent hands them; fields that each say one flag, measures that each say one number, or a number
/// the value states beside what it measures, and may disagree; a length past the field that holds
/// it where the plan checks it, past the bits of the
/// flag that holds it, or below what its arithmetic takes off; and the refusal of a body or an
/// entry whose codec refuses, which its encode throws.
fn writes_refusal(plan: &EncodePlan<'_>, kind: RefusalKind, element: &str) -> bool {
    let derived_at =
        |derived: &&crate::forge::encode_plan::Derived<'_>| derived.element.to_string() == element;
    match kind {
        RefusalKind::PresentIfMismatch => {
            plan.gate_checks.iter().any(|f| f.id == element)
                || plan.input_checks.iter().any(|check| check.at == element)
        }
        RefusalKind::InconsistentSource => {
            plan.derivations.iter().filter(derived_at).any(|derived| {
                presence_flag(&derived.element, &derived.by).is_some()
                    || printed_number(&derived.element, &derived.by).is_some_and(|number| {
                        number.measures.len() > 1
                            || matches!(
                                number.into,
                                NumberInto::Field {
                                    absent: Absent::Stated,
                                    ..
                                }
                            )
                    })
            })
        }
        RefusalKind::LengthOutOfRange => plan.derivations.iter().any(|derived| {
            let Some(number) = printed_number(&derived.element, &derived.by) else {
                return false;
            };
            let past_its_field = derived_at(&derived)
                && match &number.into {
                    NumberInto::Field { narrow, .. } => *narrow == Narrow::Checked,
                    NumberInto::Flag => true,
                };
            let below_its_arithmetic = number.measures.iter().any(|measure| {
                matches!(
                    measure.of,
                    Measured::Bytes {
                        field,
                        offset: Offset::Less { held: false, .. },
                    } if field.id == element
                )
            });
            past_its_field || below_its_arithmetic
        }),
        // A body or a list the value presents, as an element or in the arm of a gate group.
        RefusalKind::Child => {
            plan.model.fields.iter().any(|f| {
                f.id == element
                    && matches!(
                        presented_value(plan, f),
                        Some(Value::Body { .. } | Value::List { .. })
                    )
            }) || plan.arms.iter().any(|arm| arm.codec == element)
        }
        _ => false,
    }
}

/// Whether the printer writes a value of `t`: an integer of a native width, or a value of an
/// imported enum, written as its carrier.
fn is_number(t: &SceType) -> bool {
    t.int_bit_width().is_some() || matches!(t, SceType::Enum(_))
}

/// The protocol a value of the codec `codec` presents its elements through.
pub(crate) fn source_protocol(codec: &str) -> String {
    format!("{}Source", filters::to_pascal_case(codec.to_string()))
}

/// The static function of a printed codec that reads the flag input `input` off a source of it
/// (docs/adr/0065 §6 R4a): `input` and the input's name in upper camel case.
pub(crate) fn said_reader(input: &str) -> String {
    format!("input{}", filters::to_pascal_case(input.to_string()))
}

/// The associated type of a printed codec's protocol that the body `field` is a source of, which
/// the generated value binds to the body's own value: the field's name in upper camel case and
/// `_Source`. The name of a codec's struct or protocol is in upper camel case and holds no `_`, so
/// the associated type hides none of them, the body's own protocol among them.
fn body_source_type(field: &str) -> String {
    format!("{}_Source", filters::to_pascal_case(field.to_string()))
}

/// The `UInt8` the flag input `input` that the value says holds, read off `source`, a source of
/// the codec of `plan`: set exactly where the field gated on it alone is present (absent, for a
/// field gated on its clear state), what the arm the value holds of the gate group it decides
/// gives it, or what the body always there that it hands it on to says, read through the body's
/// codec. [`unlowered`] refuses every other way a value says one.
fn said_input(plan: &EncodePlan<'_>, held: &HeldTypes, input: &str, source: &str) -> String {
    match &plan.roles.inputs[input] {
        InputSource::Gate { field, when_clear } => {
            let test = if *when_clear { "==" } else { "!=" };
            format!("{source}.{field} {test} nil ? 1 : 0")
        }
        InputSource::Forward {
            embed,
            input,
            partly: false,
            ..
        } => {
            let child = held
                .get(embed)
                .expect("an input forwarded from a body is said by a body the codec holds");
            format!("{child}.{}({source}.{embed})", said_reader(input))
        }
        // What the arm the value holds says, read through the codec's own reader of it, which
        // [`said_lines`] writes as a switch over the arms.
        InputSource::EveryArm { .. } => format!("Self.{}({source})", said_reader(input)),
        // The value of the gate the arm of the group it decides gives it (docs/adr/0065 §6 R4c).
        InputSource::Group => {
            let gate = GateVar::Input(input.to_string());
            let group = plan
                .groups()
                .iter()
                .find(|group| group.gates.contains(&gate))
                .expect("an input a group says is a gate of the group");
            let at = group
                .gates
                .iter()
                .position(|g| *g == gate)
                .expect("the input is one of the group's gates");
            let held = format!("{source}.{}", group_name(group));
            format!("{} ? 1 : 0", gate_value(group, at, &held))
        }
        // The start of the list held to its law that the codec hands the input on to
        // (docs/adr/0065 §6 R8); a list a gate leaves out says nothing, which the encode reads as
        // clear and [`said_lines`] answers as `nil`.
        InputSource::List {
            list,
            input: entry_input,
            partly,
            ..
        } => {
            let path = law_start_path(plan, field_named(plan, list), entry_input);
            if group_of(plan, list).is_some() {
                // A list a gate group holds, read off the arm the value holds into its member's
                // local, which the encode binds before it reads any input.
                format!(
                    "({}.map {{ $0.start{path} ? 1 : 0 }} ?? 0)",
                    member_local(list)
                )
            } else if *partly {
                format!("({source}.{list}.map {{ $0.start{path} ? 1 : 0 }} ?? 0)")
            } else {
                format!("{source}.{list}.start{path} ? 1 : 0")
            }
        }
        InputSource::Forward { partly: true, .. } | InputSource::SomeArms { .. } => {
            unreachable!("`unlowered` refuses an input some arms say that the encode reads")
        }
    }
}

/// The statements of the function that reads the flag input `input` the value says off a source
/// of the codec of `plan` (docs/adr/0065 §6 R4a): what the arm the value holds says, read through
/// the arm's codec, and, where some arms do not read it, nothing for those (§6 R4e); or the one
/// expression [`said_input`] reads it by. `arms` are the variant's arms as the printer spells them.
fn said_lines(
    plan: &EncodePlan<'_>,
    held: &HeldTypes,
    arms: &[ArmText],
    input: &str,
) -> Vec<String> {
    let (bound, every) = match &plan.roles.inputs[input] {
        InputSource::EveryArm { arms } => (arms, true),
        InputSource::SomeArms { arms } => (arms, false),
        // A list a gate group holds says the input by its start where the arm the value holds holds
        // it, and nothing where it does not (docs/adr/0065 §6 R4e, R8).
        InputSource::List {
            list,
            input: entry_input,
            ..
        } if group_of(plan, list).is_some() => {
            return group_list_said(plan, list, entry_input);
        }
        // A list a gate leaves out says nothing of the input (docs/adr/0065 §6 R4e).
        InputSource::List {
            list,
            input: entry_input,
            partly: true,
            ..
        } => {
            let path = law_start_path(plan, field_named(plan, list), entry_input);
            return vec![format!("{SOURCE}.{list}.map {{ $0.start{path} ? 1 : 0 }}")];
        }
        _ => return vec![said_input(plan, held, input, SOURCE)],
    };
    let mut lines = vec![format!("switch {SOURCE}.{} {{", super::VARIANT_FIELD)];
    for (arm, text) in plan.arms.iter().zip(arms) {
        let line = match bound.get(arm.codec) {
            Some(said) => format!(
                "case {}: return {}.{}(arm)",
                text.pattern("arm"),
                text.child,
                said_reader(said)
            ),
            None if every => unreachable!("every arm says an input every arm says"),
            None => format!("case {}: return nil", text.bare()),
        };
        lines.extend(text.guarded(vec![line]));
    }
    lines.push("}".to_string());
    lines
}

/// The statements of the function that reads, off a source of the codec of `plan`, the flag input
/// the list `list` says by its start (docs/adr/0065 §6 R8), where a gate group holds the list: the
/// start of the input `input` of its law as 1 or 0 where the arm the value holds holds the list,
/// `nil` where it holds another arm or none (§6 R4e).
fn group_list_said(plan: &EncodePlan<'_>, list: &str, input: &str) -> Vec<String> {
    let group = &plan.groups()[group_of(plan, list).expect("the list is a member of a group")];
    let path = law_start_path(plan, field_named(plan, list), input);
    let said = |held: &str| format!("return {held}.start{path} ? 1 : 0");
    let held = format!("{SOURCE}.{}", group_name(group));
    if !group.is_sum() {
        let arm = &group.arms[0];
        if group.absent.is_none() {
            return vec![said(&format!("{held}.{list}"))];
        }
        let holds = arm.fields.iter().any(|f| f == list);
        return if holds {
            vec![
                format!(
                    "if let {HELD} = {held} {{ {} }}",
                    said(&format!("{HELD}.{list}"))
                ),
                "return nil".to_string(),
            ]
        } else {
            vec!["return nil".to_string()]
        };
    }
    let optional = if group.absent.is_some() { "?" } else { "" };
    let mut lines = vec![format!("switch {held} {{")];
    for arm in &group.arms {
        let case = group_case(group, arm);
        let members = arm_members(group, arm);
        let holds = members
            .iter()
            .any(|member| matches!(member, ArmMember::One(id) if *id == list));
        lines.push(if holds {
            let binds: Vec<&str> = members
                .iter()
                .map(|member| match member {
                    ArmMember::One(id) if *id == list => "let held",
                    _ => "_",
                })
                .collect();
            format!(
                "case .{case}({}){optional}: {}",
                binds.join(", "),
                said(HELD)
            )
        } else {
            format!("case .{case}{optional}: return nil")
        });
    }
    if group.absent.is_some() {
        lines.push("case nil: return nil".to_string());
    }
    lines.push("}".to_string());
    lines
}

/// The name a printed encode gives the source it writes.
const SOURCE: &str = "source";

/// The name a printed encode gives what a gated element holds where it is present.
const HELD: &str = "held";

/// The statement that throws the refusal of `kind` at `element`, which the totality analysis names
/// (docs/adr/0065 §2) and records as written: the runtime's `CodecError` case of the variant it
/// names, in lower camel case.
fn refusal(plan: &EncodePlan<'_>, kind: RefusalKind, element: &str) -> String {
    let variant = plan.totality.require(kind, element);
    let variant = variant.trim_start_matches("CodecError::");
    let mut chars = variant.chars();
    let case = match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect::<String>(),
        None => unreachable!("an encode's own refusal names a variant of CodecError"),
    };
    format!("throw CodecError.{case}")
}

/// The bound type of the byte string or text `field` of a printed codec: a type the codec's value
/// declares, which a protocol of the host's module names through it.
fn bound_name(field: &str) -> String {
    format!("{}Bound", filters::to_pascal_case(field.to_string()))
}

/// The Swift type a printed codec named `struct_name` holds the byte string or text `element` as,
/// within the bound its own type declares (docs/adr/0065 §6 R1); `None` for another element.
pub(super) fn bounded_type(struct_name: &str, element: &Element<'_>) -> Option<String> {
    bounded_value_type(struct_name, &element.field.id, &element.value)
}

/// The Swift type a printed codec named `struct_name` holds the field `id`, which holds `value`
/// where it is present, as: its byte string or text within the bound its own type declares
/// (docs/adr/0065 §6 R1); `None` for another value.
fn bounded_value_type(struct_name: &str, id: &str, value: &Value<'_>) -> Option<String> {
    let runtime = match value {
        Value::Bytes(_) => "BoundedBytes",
        Value::Text(_) => "BoundedText",
        _ => return None,
    };
    Some(format!("{runtime}<{struct_name}.{}>", bound_name(id)))
}

/// The associated type of a printed codec's protocol that each entry of the list or chain `field`
/// is a source of, which the generated value binds to the entry codec's own value: the field's name
/// in upper camel case and `_Entry`, which hides no codec's type, as [`body_source_type`] says.
fn entry_source_type(field: &str) -> String {
    format!("{}_Entry", filters::to_pascal_case(field.to_string()))
}

/// The Swift type a printed codec named `struct_name` holds the list or chain `id` as, each entry
/// an `entry`, within the count bound its own type declares (docs/adr/0065 §6 R2, R7), and, where
/// its entries say inputs it hands them (`law`), held to the law its own type declares (R8).
pub(super) fn list_type(struct_name: &str, id: &str, entry: &str, law: &Law) -> String {
    let list = bounded_list_type(struct_name, id, entry);
    if law.is_none() {
        list
    } else {
        format!("Lawful<{list}, {struct_name}.{}<{entry}>>", law_name(id))
    }
}

/// The kind a printed codec named `struct_name` declares for the entries of its chain `chain` of
/// the kind `key` a gate names (docs/adr/0065 §6 R6), a `SceKind` generic over the source of the
/// entries: the chain's name in upper camel case, `Kind` and the kind in hexadecimal.
fn kind_type(struct_name: &str, chain: &str, key: u64, entry: &str) -> String {
    format!("{struct_name}.{}<{entry}>", kind_name(chain, key))
}

/// The name of [`kind_type`] within the codec's struct.
fn kind_name(chain: &str, key: u64) -> String {
    format!("{}Kind{key:X}", filters::to_pascal_case(chain.to_string()))
}

/// The set of the kinds `keys` of the chain `chain` of the codec named `struct_name`, each entry an
/// `entry` (`SceKinds`): none, or the first and the rest nested, as the laws of a list nest.
fn kinds_type(struct_name: &str, chain: &str, keys: &[u64], entry: &str) -> String {
    match keys.split_first() {
        None => format!("SceNoKinds<{entry}>"),
        Some((first, rest)) => format!(
            "SceKindsOf<{}, {}>",
            kind_type(struct_name, chain, *first, entry),
            kinds_type(struct_name, chain, rest, entry)
        ),
    }
}

/// Whether `arm` of `group` holds the group's chain as a bare entry's place among the others
/// (docs/adr/0065 §6 R6a): the group's one gate is that entry's, and the arm holds it.
pub(super) fn holds_place(group: &GateGroup, arm: &GroupArm) -> bool {
    group.member_is_bare() && !group.kinds_in(arm).0.is_empty()
}

/// Whether the field `id` is the chain whose memberships are the gates of `group` (docs/adr/0065 §6
/// R6), which every arm holds in a type of the kinds it says.
pub(super) fn is_group_chain(group: &GateGroup, id: &str) -> bool {
    group.chain() == Some(id)
}

/// The Swift type `arm` of `group`, a group of the codec named `struct_name`, holds its chain
/// `field` as, each entry an `entry` (docs/adr/0065 §6 R6a, R6b): the chain whole within its count
/// bound with an entry of each kind the arm sets and of none it clears (`SceChain`), optional where
/// the chain is gated and the arm sets no kind; where the one gate on a bare entry holds, the
/// others and the entry's place (`ScePlaced`).
pub(super) fn chain_arm_type(
    struct_name: &str,
    group: &GateGroup,
    arm: &GroupArm,
    field: &CodecField,
    entry: &str,
) -> String {
    let id = &field.id;
    let bound = format!("{struct_name}.{}", bound_name(id));
    let (holds, lacks) = group.kinds_in(arm);
    if holds_place(group, arm) {
        return format!(
            "ScePlaced<{entry}, {bound}, {}>",
            kind_type(struct_name, id, holds[0], entry)
        );
    }
    let chain = format!(
        "SceChain<{entry}, {bound}, {}, {}>",
        kinds_type(struct_name, id, &holds, entry),
        kinds_type(struct_name, id, &lacks, entry)
    );
    if super::super::rust_source::chain_optional_in(group, field, arm) {
        format!("{chain}?")
    } else {
        chain
    }
}

/// The value of a list of entries each a value of the codec whose struct is `entry` that has read
/// no frame, written where its type is known: the fewest entries its bound admits, each a value of
/// a frame of zeros, and, where its entries say inputs it hands them (`law`), held to its law from
/// the start the first of them says (docs/adr/0065 §6 R8).
fn list_default(entry: &str, law: &Law) -> String {
    let fewest = format!(".fewest(of: {entry}())");
    if law.is_none() {
        fewest
    } else {
        format!(".init(defaultOf: {fewest})")
    }
}

/// The bounded list a printed codec named `struct_name` holds the entries of the list or chain
/// `id` in, each an `entry`, within the count bound its own type declares (docs/adr/0065 §6 R2,
/// R7), whether or not a law holds them too.
fn bounded_list_type(struct_name: &str, id: &str, entry: &str) -> String {
    format!("BoundedList<{entry}, {struct_name}.{}>", bound_name(id))
}

/// The law a printed codec declares for the list `id` (docs/adr/0065 §6 R8), generic over the
/// source of its entries: the field's name in upper camel case and `Law`, beside its `Bound`.
fn law_name(id: &str) -> String {
    format!("{}Law", filters::to_pascal_case(id.to_string()))
}

/// The reader a printed codec declares of what each entry of the list `id` says of the input
/// `input` (`SceSaid`), or, with `Kind`, of whether an entry is of the kind a law over that input
/// asks after (`SceKind`).
fn law_reader_name(id: &str, input: &str, what: &str) -> String {
    format!(
        "{}{}{what}",
        filters::to_pascal_case(id.to_string()),
        filters::to_pascal_case(input.to_string())
    )
}

/// Where the start of the input at `at` among the `count` inputs of a list's law is held in the
/// start the list holds (docs/adr/0065 §6 R8): the start itself for a law of one input; else, the
/// laws nested as the first and the rest (`SceLaws`), `.second` once for each input before it and
/// `.first` where it is not the last.
fn start_path(count: usize, at: usize) -> String {
    if count == 1 {
        return String::new();
    }
    let mut path = ".second".repeat(at);
    if at + 1 < count {
        path.push_str(".first");
    }
    path
}

/// Where the start of the input `input` of the law of the list `list` of `plan` is held in the
/// start the list holds ([`start_path`]): a `Bool`, read as `.start` and this path.
fn law_start_path(plan: &EncodePlan<'_>, list: &CodecField, input: &str) -> String {
    let laws = super::super::rust_source::list_laws(&plan.roles, &list.id);
    let at = laws
        .iter()
        .position(|law| law.binder.input == input)
        .expect("the list holds the input by its law");
    start_path(laws.len(), at)
}

/// The start a decode holds the entries of the list `field` of `plan` it read to (docs/adr/0065
/// §6 R8), from what the frame gave each input of the list's law, in the law's order: the bit of
/// a carrier the decode read, the input the decode is handed, and nothing for an input the entry
/// before decides, whose start is fixed; several nested as the first and the rest.
fn law_start_read(plan: &EncodePlan<'_>, field: &CodecField) -> String {
    let starts: Vec<String> = super::super::rust_source::list_laws(&plan.roles, &field.id)
        .into_iter()
        .map(|law| match &law.against {
            InputValue::Carrier { carrier, flag } => {
                let bit = field_named(plan, carrier)
                    .flags
                    .iter()
                    .find(|f| f.name == *flag)
                    .map(|f| f.bit)
                    .expect("a law's flag is a flag of the codec's carrier");
                format!("(({} >> {bit}) & 1) != 0", super::decoded(carrier))
            }
            InputValue::Input { name } => format!("({name} & 0x01) != 0"),
            InputValue::PreviousEntry { .. } => "SceFixedStart()".to_string(),
        })
        .collect();
    nested_starts(&starts)
}

/// The starts `starts` of a list's law, in its order, as the start the list holds: one alone, or
/// the first and the rest (`SceStarts`).
pub(super) fn nested_starts(starts: &[String]) -> String {
    nested(starts, "SceStarts(", ")")
}

/// The parts `parts` of a list's law, one for each input in its order, run together as the
/// runtime nests them: one alone, or `open`, the first, the rest nested so, and `close`.
fn nested(parts: &[String], open: &str, close: &str) -> String {
    match parts {
        [] => unreachable!("a list's law holds an automaton for each input"),
        [one] => one.clone(),
        [first, rest @ ..] => format!("{open}{first}, {}{close}", nested(rest, open, close)),
    }
}

/// What the template declares for each list of `plan` its entries' law holds (docs/adr/0065 §6
/// R8): for each input, a reader of what an entry says of it through the entry codec's reader of
/// the input (`SceSaid`), and for an input a chain supplies from the entry before, a reader of
/// whether an entry is of the kind that sets it (`SceKind`); and the law, each input's automaton
/// run together, generic over the source of the entries. `held` is the Swift struct of each list's
/// entries by its field; `imports` says which entry codec says an input in some of its values only.
fn law_context(
    plan: &EncodePlan<'_>,
    held: &HeldTypes,
    imports: &[ImportContext],
) -> Vec<serde_json::Value> {
    plan.model
        .fields
        .iter()
        .filter_map(|field| {
            let Some(Value::List { codec, law, .. }) = presented_value(plan, field) else {
                return None;
            };
            if law.is_none() {
                return None;
            }
            let child = held
                .get(&field.id)
                .expect("a list held to its law holds a codec of the module");
            let mut readers = Vec::new();
            let mut automata = Vec::new();
            for input in &law.inputs {
                let said = law_reader_name(&field.id, &input.input, "Said");
                let reader = format!("{child}.{}(entry)", said_reader(&input.input));
                // An entry that says the input in some of its values only says nothing in the
                // others (docs/adr/0065 §6 R4e), which follows any state of the law (R8b).
                let says = if says_partly(imports, codec, &input.input) {
                    format!("{reader}.map {{ $0 != 0 }}")
                } else {
                    format!("{reader} != 0")
                };
                readers.push(serde_json::json!({
                    "name": said,
                    "protocol": "SceSaid",
                    "function": "says",
                    "returns": "Bool?",
                    "body": says,
                    "what": format!("what each entry says of `{}`", input.input),
                }));
                automata.push(match input.automaton {
                    Automaton::Uniform => format!("SceUniform<{said}<Entry>>"),
                    Automaton::AfterKey { id } => {
                        let kind = law_reader_name(&field.id, &input.input, "Kind");
                        readers.push(serde_json::json!({
                            "name": kind,
                            "protocol": "SceKind",
                            "function": "matches",
                            "returns": "Bool",
                            "body": format!(
                                "{} == 0x{id:X}",
                                entry_identifier(plan, &field.id, "entry")
                            ),
                            "what": format!(
                                "whether an entry is of the kind 0x{id:X}, after which the next \
                                 says `{}` set",
                                input.input
                            ),
                        }));
                        format!("SceAfterKey<{said}<Entry>, {kind}<Entry>>")
                    }
                });
            }
            Some(serde_json::json!({
                "name": law_name(&field.id),
                "field": field.id,
                "protocol": format!("{child}Source"),
                "readers": readers,
                // The automata run together: one alone, or the first and the rest (`SceLaws`).
                "law": nested(&automata, "SceLaws<", ">"),
            }))
        })
        .collect()
}

/// The Swift type the protocol of `plan` presents `element` as: its number, its bounded byte string
/// or text, the associated type its body is a source of, or its list of the associated type each
/// entry is a source of, optional where a gate decides it.
fn presented_type(l: &LangCtx, plan: &EncodePlan<'_>, element: &Element<'_>) -> String {
    let struct_name = filters::to_pascal_case(plan.model.name.clone());
    let ty = match &element.value {
        Value::Body { .. } => body_source_type(&element.field.id),
        Value::List { law, .. } => list_type(
            &struct_name,
            &element.field.id,
            &entry_source_type(&element.field.id),
            law,
        ),
        _ => bounded_type(&struct_name, element)
            .unwrap_or_else(|| l.type_name(&element.field.sce_type).into_owned()),
    };
    if element.presence.optional() {
        format!("{ty}?")
    } else {
        ty
    }
}

/// What a decode makes the element `element` of `plan` of, once the frame is read: the local it
/// read the field into, and for a number the value states where what it measures is absent
/// (docs/adr/0058 §1, `Presence::Stated`), that number only where none of what it measures was in
/// the frame, `nil` where some was, as the C++ printer's decode presents it.
fn element_decoded(plan: &EncodePlan<'_>, element: &Element<'_>) -> String {
    let id = &element.field.id;
    if element.presence != Presence::Stated {
        return super::decoded(id);
    }
    let absent: Vec<String> = stated_contents(plan, id)
        .iter()
        .map(|content| format!("{} == nil", super::decoded(&content.id)))
        .collect();
    format!("({}) ? {} : nil", absent.join(" && "), super::decoded(id))
}

/// The fields the number the value states as the field `id` of `plan` measures, each gated: the
/// byte strings and texts it is the length of, the lists it counts, the bodies it is the encoded
/// length of.
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

/// One measure of a derived number as the encode reads it.
struct MeasureText {
    /// The number, an `Int`, which is read only where the measure is there.
    size: String,
    /// Where the measure may be absent, the test that it is there.
    present: Option<String>,
    /// Whether the plan says the measure may be absent (`Measure::gated`).
    gated: bool,
}

/// The local the encode holds the derived `element` in: a field's `<field>_derived`, a flag's
/// `<carrier>_<flag>_derived`. A field is read through the source and never bare, so the local
/// hides no field.
fn derived_local(element: &ElementId) -> String {
    match element {
        ElementId::Field(id) => format!("{id}_derived"),
        ElementId::Flag { field, flag } => format!("{field}_{flag}_derived"),
        ElementId::Variant => unreachable!("a variant's tag is no number of bytes"),
    }
}

/// The local the encode holds the bytes the body `field` encodes to in, `nil` where a gate leaves
/// the body out. A field is read through the source and never bare, so the local hides no field.
fn body_bytes(field: &str) -> String {
    format!("{field}_bytes")
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

/// The number the derived `named` holds, made from what it measures as the Rust encode's `combine`
/// makes it (docs/adr/0081 §1), and, where every measure may be absent, the test that one is there;
/// the statements it needs are pushed onto `body`. The first measure always there fixes it, and every
/// other one there must say the same, or the value is refused as `inconsistentSource`; where every
/// measure may be absent, it is the first one there, held in `<local>_first`.
fn combine(
    plan: &EncodePlan<'_>,
    named: &str,
    local: &str,
    measures: &[MeasureText],
    body: &mut Vec<String>,
) -> (String, Option<String>) {
    match measures {
        // Nothing measured: zero, as the Rust encode's `combine` holds it.
        [] => ("0".to_string(), None),
        [one] => (one.size.clone(), one.present.clone()),
        _ => {
            let disagree = || refusal(plan, RefusalKind::InconsistentSource, named);
            let there = |m: &MeasureText| m.present.clone().unwrap_or_else(|| "false".to_string());
            let always: Vec<&MeasureText> = measures.iter().filter(|m| !m.gated).collect();
            let gated: Vec<&MeasureText> = measures.iter().filter(|m| m.gated).collect();
            if let Some((anchor, others)) = always.split_first() {
                for other in others {
                    body.push(format!(
                        "if {} != {} {{ {} }}",
                        other.size,
                        anchor.size,
                        disagree()
                    ));
                }
                for g in &gated {
                    body.push(format!(
                        "if {} && {} != {} {{ {} }}",
                        there(g),
                        g.size,
                        anchor.size,
                        disagree()
                    ));
                }
                (anchor.size.clone(), None)
            } else {
                let first = format!("{local}_first");
                body.push(format!("var {first}: Int? = nil"));
                for (i, g) in gated.iter().enumerate() {
                    let keyword = if i == 0 { "if" } else { "} else if" };
                    body.push(format!("{keyword} {} {{", there(g)));
                    body.push(format!("    {first} = {}", g.size));
                }
                body.push("}".to_string());
                for g in gated.iter().skip(1) {
                    body.push(format!(
                        "if {} && {} != {first} {{ {} }}",
                        there(g),
                        g.size,
                        disagree()
                    ));
                }
                (format!("({first} ?? 0)"), Some(format!("{first} != nil")))
            }
        }
    }
}

/// The derived number `size` (there where `present` holds, if it may not be), held in the local of
/// the field `id` it goes `into`, of that field's type: refused where it may not fit (docs/adr/0065
/// §5); where nothing it measures is there, zero, beside which the field is gated and no write
/// reads it, what the arm the value holds of the gate group that holds the field states, zero where
/// the arm states nothing (§6 R4b, `Absent::Group`), or what the value states, zero where it states
/// nothing (docs/adr/0058 §1, `Absent::Stated`), as every printer reads it. A value that states the
/// number where something it measures is there says it twice, and is refused as
/// `inconsistentSource` where it says another number than the one derived.
fn field_number(
    l: &LangCtx,
    plan: &EncodePlan<'_>,
    id: &str,
    into: &NumberInto<'_>,
    size: &str,
    present: Option<&str>,
    body: &mut Vec<String>,
) {
    let NumberInto::Field {
        field,
        narrow,
        absent,
    } = into
    else {
        unreachable!("a number held in a field goes into that field")
    };
    let otherwise = match absent {
        Absent::Group => format!("({} ?? 0)", member_local(id)),
        Absent::Stated => format!("({SOURCE}.{id} ?? 0)"),
        Absent::Never | Absent::Zero => "0".to_string(),
    };
    let ty = l.type_name(&field.sce_type).into_owned();
    if *narrow == Narrow::Checked {
        let most = largest_of(&field.sce_type)
            .expect("a length that may not fit its field is held in an integer field");
        let past = format!("{size} > 0x{most:X}");
        let test = match present {
            Some(there) => format!("{there} && {past}"),
            None => past,
        };
        body.push(format!(
            "if {test} {{ {} }}",
            refusal(plan, RefusalKind::LengthOutOfRange, id)
        ));
    }
    let value = match present {
        None => format!("{ty}({size})"),
        Some(there) => format!("{there} ? {ty}({size}) : {otherwise}"),
    };
    let local = derived_local(&ElementId::Field(id.to_string()));
    body.push(format!("let {local}: {ty} = {value}"));
    if *absent == Absent::Stated {
        let there =
            present.expect("a number the value states is free: every measure may be absent");
        body.push(format!(
            "if {there}, let stated = {SOURCE}.{id}, stated != {local} {{ {} }}",
            refusal(plan, RefusalKind::InconsistentSource, id)
        ));
    }
}

/// What the template writes for the codec of `plan`, which [`unlowered`] has found nothing to
/// refuse in: the protocol and its requirements, one read-only property per element in
/// declaration order, the associated type each body is a source of, the flag inputs the encode is
/// handed, and the statements of `encodeSource`: each carrier with the flags the encode derives or
/// a parent sets folded in, each body encoded into bytes of its own, the checks the value is held
/// to, and the writes of the plan's program, one after another, into a frame written in one write.
/// `held` is the Swift struct of each body by its field, and `arm_types` that of each arm of the
/// variant, in the plan's order; `imports` says what each codec the value holds says.
pub(super) fn context(
    l: &LangCtx,
    plan: &EncodePlan<'_>,
    carriers: &std::collections::HashMap<String, Carrier>,
    held: &HeldTypes,
    arm_types: &[String],
    imports: &[ImportContext],
) -> serde_json::Value {
    let struct_name = filters::to_pascal_case(plan.model.name.clone());
    let arms = arm_texts(plan, arm_types);
    // What the value presents, in the order the document declares it: each element, and each gate
    // group where its first member stands (docs/adr/0065 §6 R4b), each held in a local of its own
    // name by the decode. The protocol requires them, and the initializer takes them in this order.
    let mut requirements: Vec<serde_json::Value> = presented_order(plan)
        .into_iter()
        .map(|presented| match presented {
            Presented::Element(element) => serde_json::json!({
                "name": element.field.id,
                "type": presented_type(l, plan, element),
                "local": element_decoded(plan, element),
            }),
            Presented::Group(index) => {
                let group = &plan.groups()[index];
                serde_json::json!({
                    "name": group_name(group),
                    "type": group_holds(plan, group, held),
                    "local": group_local(group),
                })
            }
        })
        .collect();
    // The variant, last of the elements, as the arms enum of the sources each arm's body is.
    if !arms.is_empty() {
        requirements.push(serde_json::json!({
            "name": super::VARIANT_FIELD,
            "type": format!(
                "{}<{}>",
                arms_enum(&struct_name),
                arms.iter().map(|a| a.assoc.as_str()).collect::<Vec<_>>().join(", ")
            ),
            "local": super::decoded(super::VARIANT_FIELD),
        }));
    }
    let held_of = |f: &CodecField| {
        held.get(&f.id)
            .expect("every body the printer writes holds a codec of the module")
    };
    // The source each body is presented as, an element or a member of a gate group: an associated
    // type the protocol requires to be a source of the body's codec, which the generated value
    // binds to the body's own value, and which a group's type is generic over.
    let sourced =
        |is: fn(&Value<'_>) -> bool, name: fn(&str) -> String| -> Vec<serde_json::Value> {
            plan.model
                .fields
                .iter()
                .filter(|field| presented_value(plan, field).is_some_and(|value| is(&value)))
                .map(|field| {
                    let child = held_of(field);
                    serde_json::json!({
                        "name": name(&field.id),
                        "field": field.id,
                        "protocol": format!("{child}Source"),
                        "type": child,
                    })
                })
                .collect()
        };
    let bodies = sourced(
        |value| matches!(value, Value::Body { .. }),
        body_source_type,
    );
    // The source each entry of a list or a chain is presented as, as a body is.
    let entries = sourced(
        |value| matches!(value, Value::List { .. }),
        entry_source_type,
    );
    // The bound each byte string and text is held to, a type of the codec's own (docs/adr/0065 §6
    // R1), and the count bound each list or chain is held to (R2, R7): the value's type of the
    // element names it, so a value made for one is no value of another, and a byte bound is no
    // count bound.
    // A byte string or a text a gate group holds is held to its bound in the arm that holds it.
    let bounds: Vec<serde_json::Value> = plan
        .model
        .fields
        .iter()
        .filter_map(|field| {
            let presented = plan.elements.iter().any(|e| e.field.id == field.id)
                || group_of(plan, &field.id).is_some();
            if !presented {
                return None;
            }
            let value = plan.value(field);
            let (kind, bound) = match &value {
                Value::Bytes(bound) | Value::Text(bound) => ("SceByteBound", bound),
                Value::List { bound, .. } => ("SceCountBound", bound),
                _ => return None,
            };
            let Bound { least, most } = *bound;
            Some(serde_json::json!({
                "name": bound_name(&field.id),
                "field": field.id,
                "protocol": kind,
                "least": least,
                "most": most.map_or_else(|| "nil".to_string(), |most| most.to_string()),
            }))
        })
        .collect();
    let carrier_of = |f: &CodecField| {
        carriers
            .get(&f.id)
            .expect("every field the printer writes holds a number")
    };
    let field_of = |id: &str| {
        plan.model
            .fields
            .iter()
            .find(|f| f.id == id)
            .expect("a derived flag is a flag of a field of the codec")
    };
    let mut body: Vec<String> = Vec::new();
    // Each member of each gate group, read off the arm the value holds into a local of its own,
    // `nil` where that arm does not hold it (docs/adr/0065 §6 R4b): what a presence flag, a length
    // and a write read the member by, as they read an element off the source.
    for group in plan.groups() {
        body.extend(group_member_lines(l, plan, group));
    }
    // Each input the value says (docs/adr/0065 §6 R4a) that a gate of the codec reads, or that the
    // codec hands a body or holds a body to, read off what the value holds under the name the gate
    // reads it by; one nothing reads is left to the parent that asks for it.
    for said in plan
        .said
        .iter()
        .filter(|said| said.read || handed_on(plan, &said.input.name))
    {
        body.push(format!(
            "let {}: UInt8 = {}",
            said.input.name,
            said_input(plan, held, &said.input.name, SOURCE)
        ));
    }
    // Each carrier the encode writes otherwise than the value presents it, folded into a local
    // first: the flags its presence derives, then those a parent sets in place of the value's own.
    let mut folded: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut fold = |carrier: &str, body: &mut Vec<String>| {
        let local = folded_carrier(carrier);
        if folded.insert(carrier.to_string()) {
            body.push(format!("var {local} = {SOURCE}.{carrier}"));
        }
        local
    };
    // Each number the value's elements say, which a field holds or a flag of a carrier, made from
    // what it measures (docs/adr/0058 §1): the length of a byte string or a text, and the input a
    // body says. The encoded length of a body is counted once the body is encoded, below.
    let mut below: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut derived_fields: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for derived in &plan.derivations {
        let Some(number) = printed_number(&derived.element, &derived.by) else {
            continue;
        };
        if is_encoded_length(number) {
            continue;
        }
        let named = derived.element.to_string();
        let measures: Vec<MeasureText> = number
            .measures
            .iter()
            .map(|measure| match &measure.of {
                Measured::Bytes { field, offset } => {
                    let content = presented_read(plan, &field.id);
                    let (raw, present) = if measure.gated {
                        (
                            format!("({content}?.byteCount ?? 0)"),
                            Some(format!("{content} != nil")),
                        )
                    } else {
                        (format!("{content}.byteCount"), None)
                    };
                    // A wire that holds the count less what its arithmetic takes off holds no
                    // count below that (docs/adr/0065 §5).
                    if let Offset::Less { by, held: false } = offset {
                        if plan
                            .totality
                            .refuses(RefusalKind::LengthOutOfRange, &field.id)
                            && below.insert(field.id.clone())
                        {
                            let test = match &present {
                                Some(there) => format!("{there} && {raw} < {by}"),
                                None => format!("{raw} < {by}"),
                            };
                            body.push(format!(
                                "if {test} {{ {} }}",
                                refusal(plan, RefusalKind::LengthOutOfRange, &field.id)
                            ));
                        }
                    }
                    let size = match offset {
                        Offset::None => raw,
                        Offset::Less { by, .. } => format!("({raw} - {by})"),
                        Offset::More { by } => format!("({raw} + {by})"),
                    };
                    MeasureText {
                        size,
                        present,
                        gated: measure.gated,
                    }
                }
                // How many entries a list or a chain holds, as the value presents it within its
                // bound (docs/adr/0065 §6 R2), an element or a member of a gate group; where a gate
                // leaves the list out, it counts none.
                Measured::Entries { list } => {
                    let content = presented_read(plan, &list.id);
                    if measure.gated {
                        MeasureText {
                            size: format!("({content}?.count ?? 0)"),
                            present: Some(format!("{content} != nil")),
                            gated: true,
                        }
                    } else if group_of(plan, &list.id).is_some() {
                        // A list every arm of a group holds (the first of two lists of one count,
                        // docs/adr/0065 §6 R5) is read off the arm into a local that the arm
                        // always sets.
                        MeasureText {
                            size: format!("({content}?.count ?? 0)"),
                            present: None,
                            gated: false,
                        }
                    } else {
                        MeasureText {
                            size: format!("{content}.count"),
                            present: None,
                            gated: false,
                        }
                    }
                }
                // The input a body says, read off the source the value presents it by through its
                // codec (docs/adr/0065 §6 R4a); where a gate leaves the body out, it says nothing.
                Measured::Says {
                    body: holder,
                    input,
                    ..
                } => {
                    let child = held_of(holder);
                    let reader = said_reader(input);
                    let content = presented_read(plan, &holder.id);
                    if measure.gated {
                        MeasureText {
                            size: format!("({content}.map {{ Int({child}.{reader}($0)) }} ?? 0)"),
                            present: Some(format!("{content} != nil")),
                            gated: true,
                        }
                    } else {
                        MeasureText {
                            size: format!("Int({child}.{reader}({content}))"),
                            present: None,
                            gated: false,
                        }
                    }
                }
                // The input the arm the value holds says, read through the arm's codec off the
                // source the arm presents its body as; nothing where the value holds an arm that
                // does not say it (docs/adr/0065 §6 R4a).
                Measured::ArmSays { arms: saying } => {
                    let said = format!("{}_said{}", derived_local(&derived.element), measure.at);
                    body.push(format!("var {said}: Int? = nil"));
                    for arm_input in saying {
                        let text = &arms[arm_input.arm];
                        body.extend(text.guarded(vec![format!(
                            "if case {} = {SOURCE}.{} {{ {said} = Int({}.{}(arm)) }}",
                            text.pattern("arm"),
                            super::VARIANT_FIELD,
                            text.child,
                            said_reader(&arm_input.input)
                        )]));
                    }
                    MeasureText {
                        size: format!("({said} ?? 0)"),
                        present: measure.gated.then(|| format!("{said} != nil")),
                        gated: measure.gated,
                    }
                }
                // The start of the list held to its law that the flag is bound to (docs/adr/0065
                // §6 R8), an element or a member of a gate group; where a gate leaves the list out,
                // or the value holds an arm that does not hold it, there is none, and a flag keeps
                // the bits the value presents.
                Measured::Start { list, input } => {
                    let content = presented_read(plan, &list.id);
                    let path = law_start_path(plan, list, input);
                    if !measure.gated && group_of(plan, &list.id).is_some() {
                        // A list every arm of a group holds (the first of two lists of one count,
                        // docs/adr/0065 §6 R5) is read off the arm into a local that the arm always
                        // sets.
                        MeasureText {
                            size: format!("({content}.map {{ $0.start{path} ? 1 : 0 }} ?? 0)"),
                            present: None,
                            gated: false,
                        }
                    } else if measure.gated {
                        MeasureText {
                            size: format!("({content}.map {{ $0.start{path} ? 1 : 0 }} ?? 0)"),
                            present: Some(format!("{content} != nil")),
                            gated: true,
                        }
                    } else {
                        MeasureText {
                            size: format!("({content}.start{path} ? 1 : 0)"),
                            present: None,
                            gated: false,
                        }
                    }
                }
                Measured::Encoded { .. } => {
                    unreachable!("an encoded length is counted once the bodies are encoded")
                }
            })
            .collect();
        let local = derived_local(&derived.element);
        let (size, present) = combine(plan, &named, &local, &measures, &mut body);
        match (&derived.element, &number.into) {
            (ElementId::Field(id), into @ NumberInto::Field { .. }) => {
                field_number(l, plan, id, into, &size, present.as_deref(), &mut body);
                derived_fields.insert(id.clone());
            }
            (ElementId::Flag { field, flag }, NumberInto::Flag) => {
                let def = field_of(field)
                    .flags
                    .iter()
                    .find(|def| def.name == *flag)
                    .expect("the parser found the flag a number names");
                let width = def.width.max(1);
                let bits = if width >= 64 {
                    u64::MAX
                } else {
                    (1u64 << width) - 1
                };
                if plan.totality.refuses(RefusalKind::LengthOutOfRange, &named) {
                    let past = format!("{size} > 0x{bits:X}");
                    let test = match &present {
                        Some(there) => format!("{there} && {past}"),
                        None => past,
                    };
                    body.push(format!(
                        "if {test} {{ {} }}",
                        refusal(plan, RefusalKind::LengthOutOfRange, &named)
                    ));
                }
                let carrier_type = &carrier_of(field_of(field)).integer;
                let carrier = fold(field, &mut body);
                let mask = bits << def.bit;
                let set = format!(
                    "{carrier} = ({carrier} & ~0x{mask:X}) | (({carrier_type}(truncatingIfNeeded: {size}) & 0x{bits:X}) << {})",
                    def.bit
                );
                // Where every measure may be absent and none is there, the flag keeps the bits
                // the value presents.
                body.push(match &present {
                    Some(there) => format!("if {there} {{ {set} }}"),
                    None => set,
                });
            }
            _ => unreachable!("`printed_number` holds a number into its own kind of element"),
        }
    }
    // A flag set exactly where what it gates is present (clear, for a field gated on its clear
    // state). Fields that each say it must say the same, unless one group holds them all.
    for derived in &plan.derivations {
        let Some((carrier, flag, presence)) = presence_flag(&derived.element, &derived.by) else {
            continue;
        };
        let def = field_of(carrier)
            .flags
            .iter()
            .find(|def| def.name == flag)
            .expect("the parser found the flag a gate reads");
        let says = |(gated, when_clear): &(&CodecField, bool)| {
            let test = if *when_clear { "==" } else { "!=" };
            format!("({} {test} nil)", presented_read(plan, &gated.id))
        };
        let (first, rest) = presence
            .of
            .split_first()
            .expect("a presence flag gates a field");
        if !presence.together {
            for other in rest {
                body.push(format!(
                    "if {} != {} {{ {} }}",
                    says(other),
                    says(first),
                    refusal(
                        plan,
                        RefusalKind::InconsistentSource,
                        &derived.element.to_string()
                    )
                ));
            }
        }
        let width = def.width.max(1);
        let mask = if width >= 64 {
            u64::MAX
        } else {
            ((1u64 << width) - 1) << def.bit
        };
        let local = fold(carrier, &mut body);
        body.push(format!(
            "{local} = ({local} & ~0x{mask:X}) | ({} ? 0x{mask:X} : 0)",
            says(first)
        ));
    }
    // A flag of a disjunction that decides a group: what the arm of the group the value holds gives
    // it, and what the gates are where it holds none (docs/adr/0065 §6 R4c).
    for derived in &plan.derivations {
        let (ElementId::Flag { field, flag }, Derive::ArmFlag { group, gate }) =
            (&derived.element, &derived.by)
        else {
            continue;
        };
        let group = &plan.groups()[*group];
        let def = field_of(field)
            .flags
            .iter()
            .find(|def| def.name == *flag)
            .expect("the parser found the flag a gate reads");
        let mask = 1u64 << def.bit;
        let held = format!("{SOURCE}.{}", group_name(group));
        let local = fold(field, &mut body);
        body.push(format!(
            "{local} = ({local} & ~0x{mask:X}) | ({} ? 0x{mask:X} : 0)",
            gate_value(group, *gate, &held)
        ));
    }
    // The statement that sets the bits of the flag `flag` of the carrier `carrier`, folded into
    // `local`, to the number `value`, keeping the carrier's other bits.
    let flag_set = |carrier: &str, flag: &str, local: &str, value: &str| {
        let def = field_of(carrier)
            .flags
            .iter()
            .find(|def| def.name == flag)
            .expect("the parser found the flag a tag fills");
        let width = def.width.max(1);
        let bits = if width >= 64 {
            u64::MAX
        } else {
            (1u64 << width) - 1
        };
        let carrier_type = &carrier_of(field_of(carrier)).integer;
        format!(
            "{local} = ({local} & ~0x{:X}) | (({carrier_type}(truncatingIfNeeded: {value}) & 0x{bits:X}) << {})",
            bits << def.bit,
            def.bit
        )
    };
    // The tag of the arm the value holds, read off it through the arms enum's `tag`: into the
    // field the tag is, or into the bits of the flag it is (docs/adr/0081 §3).
    for derived in &plan.derivations {
        if !matches!(derived.by, Derive::VariantTag) {
            continue;
        }
        let tag = format!("{SOURCE}.{}.tag", super::VARIANT_FIELD);
        match &derived.element {
            ElementId::Field(id) => {
                let ty = l.type_name(&field_of(id).sce_type).into_owned();
                body.push(format!(
                    "let {}: {ty} = {ty}(truncatingIfNeeded: {tag})",
                    derived_local(&derived.element)
                ));
                derived_fields.insert(id.clone());
            }
            ElementId::Flag { field, flag } => {
                let carrier = fold(field, &mut body);
                body.push(flag_set(field, flag, &carrier, &tag));
            }
            ElementId::Variant => {
                unreachable!(
                    "a variant whose caller hands its tag derives none: the caller writes it"
                )
            }
        }
    }
    // The bits of a carrier the arm of each embedded body's variant fills: the tag of the arm the
    // body holds, masked to the bits, joined bit by bit; nothing where a gate leaves the body out.
    for derived in &plan.derivations {
        let (ElementId::Flag { field, flag }, Derive::ParentTag(parent)) =
            (&derived.element, &derived.by)
        else {
            continue;
        };
        let local = format!("{field}_{flag}_filled");
        body.push(format!("var {local}: UInt64 = 0"));
        for term in &parent.terms {
            let id = &term.body.id;
            let fill = |body: &str| {
                format!(
                    "{local} |= UInt64({body}.{}.tag) & 0x{:X}",
                    super::VARIANT_FIELD,
                    term.mask
                )
            };
            // The body as the value presents it, an element or a member of a gate group.
            let read = presented_read(plan, id);
            if presented_optional(plan, id) {
                body.push(format!("if let {HELD} = {read} {{ {} }}", fill(HELD)));
            } else {
                body.push(fill(&read));
            }
        }
        let carrier = fold(field, &mut body);
        body.push(flag_set(field, flag, &carrier, &local));
    }
    // The one-bit flags a parent may set in place of the ones the value presents (the flag a chain's
    // entries end on), each a member of `Overrides`: what the parent says is what the wire holds.
    let mut overrides = Vec::new();
    for &(carrier, flag) in &plan.overridable {
        let member = override_member(&carrier.id, &flag.name);
        let local = fold(&carrier.id, &mut body);
        let mask = format!("0x{:X}", 1u64 << flag.bit);
        body.push(format!(
            "if let set = overrides.{member} {{ {local} = set ? ({local} | {mask}) : ({local} & ~{mask}) }}"
        ));
        overrides.push(serde_json::json!({
            "member": member,
            "flag": flag.name,
            "carrier": carrier.id,
        }));
    }
    // A carrier as the encode writes it: the local its flags are folded into, else the source's.
    let carrier_read = |f: &CodecField| {
        if folded.contains(&f.id) {
            folded_carrier(&f.id)
        } else {
            format!("{SOURCE}.{}", f.id)
        }
    };
    // Each input a body or the entries of a list say that the parent hands them from elsewhere,
    // held to what the parent hands them, before the first byte (docs/adr/0065 §6 R4a): a value
    // whose body or entry says otherwise has no frame.
    for check in &plan.input_checks {
        body.extend(input_check(plan, held, &arms, check, &carrier_read));
    }
    // The arguments the codec imported as `codec` is handed after the sink, one for each input it
    // takes and does not say, in the order it declares them: the bits of a carrier as the encode
    // writes it, an input of the parent's own, or, for an entry of a chain, the local the chain
    // keeps what the entry before supplies in (`supplied`).
    let handed_arguments = |codec: &str, supplied: &dyn Fn(&str) -> Option<String>| -> String {
        plan.handed
            .get(codec)
            .into_iter()
            .flatten()
            .map(|handed| {
                let value = match &handed.from {
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
                            "UInt8(truncatingIfNeeded: ({} >> {bit}) & 0x{mask:X})",
                            carrier_read(carrier)
                        )
                    }
                    Some(HandedFrom::Input { name }) => name.clone(),
                    None => supplied(&handed.input).unwrap_or_else(|| {
                        unreachable!("`unlowered` refuses an input no declaration binds")
                    }),
                };
                format!(", {}: {value}", handed.input)
            })
            .collect()
    };
    // Each body, an element or a member of a gate group, encoded into bytes of its own by its codec,
    // handed the inputs it does not say: its encoded length is the count of those bytes, measured
    // once (docs/adr/0058 §3).
    for field in &plan.model.fields {
        let Some(Value::Body { codec }) = presented_value(plan, field) else {
            continue;
        };
        let id = &field.id;
        let child = held_of(field);
        let arguments = handed_arguments(codec, &|_| None);
        // A body whose codec refuses a value throws its refusal, which this encode passes on.
        if plan.totality.refuses(RefusalKind::Child, id) {
            plan.totality.require(RefusalKind::Child, id);
        }
        let bytes = body_bytes(id);
        let encode = |value: &str, piece: &str| {
            format!("try {child}.encodeSource({value}, into: &{piece}{arguments})")
        };
        let read = presented_read(plan, id);
        if presented_optional(plan, id) {
            body.push(format!("var {bytes}: [UInt8]? = nil"));
            body.push(format!("if let {HELD} = {read} {{"));
            body.push("    var piece = BytesSink()".to_string());
            body.push(format!("    {}", encode(HELD, "piece")));
            body.push(format!("    {bytes} = piece.bytes"));
            body.push("}".to_string());
        } else {
            let piece = format!("{id}_piece");
            body.push(format!("var {piece} = BytesSink()"));
            body.push(encode(&read, &piece));
            body.push(format!("let {bytes} = {piece}.bytes"));
        }
    }
    // The encoded length of each body a field holds, the count of the bytes it encoded to.
    for derived in &plan.derivations {
        let Some(number) = printed_number(&derived.element, &derived.by) else {
            continue;
        };
        if !is_encoded_length(number) {
            continue;
        }
        let measures: Vec<MeasureText> = number
            .measures
            .iter()
            .map(|measure| {
                let Measured::Encoded { body: holder, .. } = &measure.of else {
                    unreachable!(
                        "`printed_number` holds an encoded length that measures nothing else"
                    )
                };
                let bytes = body_bytes(&holder.id);
                if measure.gated {
                    MeasureText {
                        size: format!("({bytes}?.count ?? 0)"),
                        present: Some(format!("{bytes} != nil")),
                        gated: true,
                    }
                } else {
                    MeasureText {
                        size: format!("{bytes}.count"),
                        present: None,
                        gated: false,
                    }
                }
            })
            .collect();
        let named = derived.element.to_string();
        let local = derived_local(&derived.element);
        let (size, present) = combine(plan, &named, &local, &measures, &mut body);
        let ElementId::Field(id) = &derived.element else {
            unreachable!("`printed_number` holds an encoded length into a field")
        };
        field_number(
            l,
            plan,
            id,
            &number.into,
            &size,
            present.as_deref(),
            &mut body,
        );
        derived_fields.insert(id.clone());
    }
    // A field as the encode reads it: the number it derives, the carrier the flags are folded into,
    // else the source's.
    let read = |f: &CodecField| {
        if derived_fields.contains(&f.id) {
            derived_local(&ElementId::Field(f.id.clone()))
        } else {
            carrier_read(f)
        }
    };
    // A gate, as the encode writes the carriers it reads and is handed the inputs it reads.
    let gate_test = |f: &CodecField| {
        let gate = f
            .present_if
            .as_ref()
            .expect("a field held to its gate is gated");
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
                Clause::Input { name, negate } => {
                    format!("{name} {} 0", if *negate { "==" } else { "!=" })
                }
                // A membership a group's gate is: what the arm the value holds says of it
                // (docs/adr/0065 §6 R6b). Any other: whether the chain the value holds has an
                // entry of the kind, each entry told apart as the chain's walk tells it; a chain
                // a gate leaves out, or an arm does not hold, has none.
                Clause::Member {
                    chain,
                    id,
                    negate,
                    held,
                } => {
                    let holds = match held {
                        Some((at, Some(gate))) => {
                            let group = &plan.groups()[*at];
                            gate_value(group, *gate, &format!("{SOURCE}.{}", group_name(group)))
                        }
                        _ => {
                            let content = presented_read(plan, &chain.id);
                            let found = format!(
                                "contains(where: {{ _e in {} == 0x{id:X} }})",
                                entry_identifier(plan, &chain.id, "_e")
                            );
                            if presented_optional(plan, &chain.id) {
                                format!("({content}?.{found} ?? false)")
                            } else {
                                format!("{content}.{found}")
                            }
                        }
                    };
                    if *negate {
                        format!("!({holds})")
                    } else {
                        holds
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" || ")
    };
    // A field whose gate the encode cannot derive (it reads a flag input the caller hands) is
    // there exactly where its gate holds, or the value has no frame.
    for f in &plan.gate_checks {
        body.push(format!(
            "if ({}) != ({} != nil) {{ {} }}",
            gate_test(f),
            presented_read(plan, &f.id),
            refusal(plan, RefusalKind::PresentIfMismatch, &f.id)
        ));
    }
    // A frame laid out by byte is written into zeros as long as its fixed part, and the bytes or
    // text after it appended; one written in turn grows as each field is appended.
    let streamed = plan
        .program
        .iter()
        .any(|write| matches!(write, Write::Streamed { .. } | Write::Variant));
    let fixed_end = plan
        .program
        .iter()
        .filter_map(|write| match write {
            Write::Whole(f) => BitSpan::of(f, plan.model.default_endian).map(|span| span.end()),
            Write::Byte { at, .. } => Some(at + 1),
            _ => None,
        })
        .max();
    let tails = plan
        .program
        .iter()
        .any(|write| matches!(write, Write::Tail { .. }));
    body.push(match (streamed, fixed_end, tails) {
        (true, _, _) | (false, None, true) => "var frame: [UInt8] = []".to_string(),
        (false, Some(end), true) => format!("var frame = [UInt8](repeating: 0, count: {end})"),
        (false, Some(_), false) => {
            "var frame = [UInt8](repeating: 0, count: Self.frameLength)".to_string()
        }
        (false, None, false) => {
            "let frame = [UInt8](repeating: 0, count: Self.frameLength)".to_string()
        }
    });
    for write in &plan.program {
        match *write {
            Write::Whole(f) => body.extend(whole_write(
                l,
                f,
                carrier_of(f),
                &read(f),
                plan.model.default_endian,
            )),
            Write::Byte { at, ref pieces } => {
                for &(f, piece) in pieces {
                    body.push(piece_write(l, carrier_of(f), &read(f), at, piece));
                }
            }
            // A list or a chain, where the value holds it: each entry written in turn by its
            // codec's encode into bytes of the list's own, appended once every entry is written.
            Write::Streamed {
                field: f,
                guard,
                list: Some(ref walk),
            } => {
                let Some(Value::List { codec, .. }) = presented_value(plan, f) else {
                    unreachable!("a list the program walks is a list the value presents")
                };
                // A list whose entries' codec refuses a value throws its refusal, which this
                // encode passes on.
                if plan.totality.refuses(RefusalKind::Child, &f.id) {
                    plan.totality.require(RefusalKind::Child, &f.id);
                }
                // An input the chain supplies each entry from the entry before is the local the
                // walk keeps it in.
                let arguments = handed_arguments(codec, &|input: &str| {
                    walk.previous
                        .iter()
                        .any(|(bound, _)| bound == input)
                        .then(|| previous_local(&f.id, input))
                });
                // A chain of a bare entry's place is walked as slots: the others in order and the
                // entry of the kind where the value holds its place (docs/adr/0065 §6 R6a).
                let write = |entries: &str| match walk.slots {
                    Some(kind) => {
                        let named = named_entry(
                            import_of(imports, codec)
                                .expect("`unlowered` prints a chain of an import it reads"),
                            held_of(f),
                            kind,
                        )
                        .expect("`unlowered` prints a bare entry it can make of its kind");
                        slots_write(plan, f, walk, held_of(f), entries, &arguments, &named)
                    }
                    None => list_write(plan, f, walk, held_of(f), entries, &arguments),
                };
                // The list as the value presents it, an element or a member of a gate group.
                let entries = presented_read(plan, &f.id);
                match guard {
                    Guard::Always => body.extend(write(entries.as_str())),
                    Guard::Present => {
                        body.push(format!("if let {HELD} = {entries} {{"));
                        body.extend(write(HELD).into_iter().map(|line| format!("    {line}")));
                        body.push("}".to_string());
                    }
                    Guard::Gate => unreachable!("a list is presented, never derived whole"),
                }
            }
            // A body appends the bytes it encoded to, where the value holds it.
            Write::Streamed {
                field: f, guard, ..
            } if matches!(f.bit_size, BitSize::Embed) => {
                let bytes = body_bytes(&f.id);
                match guard {
                    Guard::Always => body.push(format!("frame += {bytes}")),
                    Guard::Present => {
                        body.push(format!("if let {HELD} = {bytes} {{ frame += {HELD} }}"))
                    }
                    Guard::Gate => unreachable!("a body is presented, never derived"),
                }
            }
            Write::Streamed {
                field: f, guard, ..
            } => {
                // A byte string or a text appends its bytes; a number, those of its carrier or its
                // chain.
                let append = |value: &str| {
                    if matches!(f.bit_size, BitSize::Tail | BitSize::LengthRef) {
                        format!("{value}.append(to: &frame)")
                    } else {
                        super::streamed_append(
                            l,
                            f,
                            carrier_of(f),
                            value,
                            plan.model.default_endian,
                            None,
                        )
                    }
                };
                match guard {
                    Guard::Always => body.push(append(&read(f))),
                    // Where the value holds it, as an element or in the arm of its group.
                    Guard::Present => {
                        body.push(format!(
                            "if let {HELD} = {} {{",
                            presented_read(plan, &f.id)
                        ));
                        body.push(format!("    {}", append(HELD)));
                        body.push("}".to_string());
                    }
                    // Derived whole and gated: where its gate holds, as the carriers the encode
                    // writes say.
                    Guard::Gate => {
                        body.push(format!("if {} {{", gate_test(f)));
                        body.push(format!("    {}", append(&read(f))));
                        body.push("}".to_string());
                    }
                }
            }
            // The bytes or text after a frame laid out by byte, after zeros up to the field's
            // offset where the layout aligns its fields.
            Write::Tail { field: f, pad } => {
                if pad {
                    body.push(format!(
                        "while frame.count < {} {{ frame.append(0) }}",
                        f.byte_offset
                    ));
                }
                body.push(format!("{SOURCE}.{}.append(to: &frame)", f.id));
            }
            // The arm the value holds, written by its codec's encode over the source its body is
            // presented as, handed what the arm's import binds; the catch-all, over its body.
            Write::Variant => {
                let written = format!("{}_arm", super::VARIANT_FIELD);
                body.push(format!("var {written} = BytesSink()"));
                body.push(format!("switch {SOURCE}.{} {{", super::VARIANT_FIELD));
                for (arm, text) in plan.arms.iter().zip(&arms) {
                    // An arm whose codec refuses a value throws its refusal, which this encode
                    // passes on.
                    if plan.totality.refuses(RefusalKind::Child, arm.codec) {
                        plan.totality.require(RefusalKind::Child, arm.codec);
                    }
                    body.extend(text.guarded(vec![
                        format!("case {}:", text.pattern("arm")),
                        format!(
                            "    try {}.encodeSource(arm, into: &{written}{})",
                            text.child,
                            handed_arguments(arm.codec, &|_| None)
                        ),
                    ]));
                }
                body.push("}".to_string());
                body.push(format!("frame += {written}.bytes"));
            }
        }
    }
    plan.totality.assert_all_written(&plan.model.name);
    // Each input the value says, read off any source of it by a function of its own: what a
    // parent that holds this codec derives the flag it binds to the input from, and what a log
    // over it stores (docs/adr/0065 §6 R4a).
    let said: Vec<serde_json::Value> = plan
        .said
        .iter()
        .map(|said| {
            serde_json::json!({
                "input": said.input.name,
                "reader": said_reader(&said.input.name),
                "lines": said_lines(plan, held, &arms, &said.input.name),
                "partly": plan.roles.says_input_partly(&said.input.name),
            })
        })
        .collect();
    // The arms of the variant, each the generic parameter of the arms enum its body is, bound by
    // the protocol to a source of the arm's codec (docs/adr/0058 §1).
    let arms_json: Vec<serde_json::Value> = arms
        .iter()
        .map(|arm| {
            serde_json::json!({
                "case": arm.case,
                "generic": arm.generic,
                "assoc": arm.assoc,
                "protocol": format!("{}Source", arm.child),
                "type": arm.child,
                "off": arm.off,
                "tag": arm.tag.map(|tag| format!("0x{tag:X}")),
            })
        })
        .collect();
    let groups: Vec<serde_json::Value> = plan
        .groups()
        .iter()
        .map(|group| group_context(l, plan, group, held))
        .collect();
    // Each kind of entry a group's gate asks its chain for (docs/adr/0065 §6 R6), a `SceKind` of
    // the codec's own over any source of the entries, told apart as the chain's walk tells them.
    let kinds: Vec<serde_json::Value> = plan
        .groups()
        .iter()
        .filter_map(|group| Some((group, group.chain()?)))
        .flat_map(|(group, chain)| {
            let child = held
                .get(chain)
                .expect("a chain a group holds holds a codec of the module");
            group.members().map(move |(_, key)| {
                serde_json::json!({
                    "name": kind_name(chain, key),
                    "field": chain,
                    "key": format!("0x{key:X}"),
                    "protocol": format!("{child}Source"),
                    "body": format!("{} == 0x{key:X}", entry_identifier(plan, chain, "entry")),
                })
            })
        })
        .collect();
    serde_json::json!({
        "protocol": source_protocol(&plan.model.name),
        "requirements": requirements,
        "groups": groups,
        "bodies": bodies,
        "entries": entries,
        "said": said,
        "given": plan.given.iter().map(|input| input.name.clone()).collect::<Vec<_>>(),
        "refuses": !plan.totality.is_total(),
        "bounds": bounds,
        // The law each list whose entries say the inputs it hands them is held to (docs/adr/0065
        // §6 R8), a type of the codec's own beside its count bound.
        "laws": law_context(plan, held, imports),
        "kinds": kinds,
        "body": body,
        "overrides": overrides,
        "arms": arms_json,
        "arms_enum": arms_enum(&struct_name),
        "unknown_tag": super::unknown_tag_type(&plan.model.name),
        "tag_type": l
            .type_name(&crate::forge::codec_totality::tag_type(plan.model))
            .into_owned(),
    })
}

/// The statements that hold the input a body or every entry of a list says to what the parent
/// hands it from elsewhere ([`InputCheck`], docs/adr/0065 §6 R4a), in a scope of their own: the
/// value is refused with `presentIfMismatch` where its body or an entry says otherwise. A body
/// that says the input in some of its values only disagrees with nothing in the others (§6 R4e).
/// The entries of a chain are each handed whether the entry before is of a kind, the first none,
/// which the check walks the chain for as the encode will (`Against::PreviousEntry`).
fn input_check(
    plan: &EncodePlan<'_>,
    held: &HeldTypes,
    arms: &[ArmText],
    check: &InputCheck<'_>,
    carrier_read: &dyn Fn(&CodecField) -> String,
) -> Vec<String> {
    let reader = said_reader(&check.input);
    let says_otherwise = |child: &str, value: &str| {
        if check.partly {
            format!("({child}.{reader}({value}).map {{ ($0 != 0) != handed }} ?? false)")
        } else {
            format!("(({child}.{reader}({value}) != 0) != handed)")
        }
    };
    let refuse = refusal(plan, RefusalKind::PresentIfMismatch, &check.at);
    let holder = match &check.site {
        CheckSite::Embed { body } => *body,
        CheckSite::List { list } => *list,
        // The arm the value holds, where it is one of the arms whose codec says the input.
        CheckSite::Arm { arms: checked } => {
            let handed = match &check.against {
                Against::Carrier { carrier, bit } => {
                    format!("(({} >> {bit}) & 1) != 0", carrier_read(carrier))
                }
                Against::Input { name } => format!("{name} != 0"),
                Against::PreviousEntry { .. } => {
                    unreachable!("an arm is handed no entry of a chain")
                }
            };
            let mut lines = vec![
                "do {".to_string(),
                format!("    let handed: Bool = {handed}"),
            ];
            for &i in checked {
                let text = &arms[i];
                lines.extend(text.guarded(vec![format!(
                    "    if case {} = {SOURCE}.{}, {} {{ {refuse} }}",
                    text.pattern("arm"),
                    super::VARIANT_FIELD,
                    says_otherwise(&text.child, "arm")
                )]));
            }
            lines.push("}".to_string());
            return lines;
        }
    };
    let child = held
        .get(&holder.id)
        .expect("a body or a list whose input is checked holds a codec of the module");
    let says_otherwise = |value: &str| says_otherwise(child, value);
    let before = format!("{}_before", holder.id);
    let handed = match &check.against {
        Against::Carrier { carrier, bit } => {
            format!("(({} >> {bit}) & 1) != 0", carrier_read(carrier))
        }
        Against::Input { name } => format!("{name} != 0"),
        Against::PreviousEntry { id } => format!("{before} == 0x{id:X}"),
    };
    // The body or the list as the value presents it, an element or a member of a gate group.
    let read = presented_read(plan, &holder.id);
    let gated = presented_optional(plan, &holder.id);
    let content = if gated {
        HELD.to_string()
    } else {
        read.clone()
    };
    let mut lines = vec!["do {".to_string()];
    match &check.site {
        CheckSite::List { .. } => {
            let indent = if gated { "    " } else { "" };
            let previous = matches!(check.against, Against::PreviousEntry { .. });
            if previous {
                lines.push(format!("    var {before}: UInt64? = nil"));
            }
            if gated {
                lines.push(format!("    if let {HELD} = {read} {{"));
            }
            lines.push(format!("    {indent}for entry in {content} {{"));
            lines.push(format!("    {indent}    let handed: Bool = {handed}"));
            lines.push(format!(
                "    {indent}    if {} {{ {refuse} }}",
                says_otherwise("entry")
            ));
            if previous {
                lines.push(format!(
                    "    {indent}    {before} = {}",
                    entry_identifier(plan, &holder.id, "entry")
                ));
            }
            lines.push(format!("    {indent}}}"));
            if gated {
                lines.push("    }".to_string());
            }
        }
        _ => {
            let differs = if gated {
                format!("let {HELD} = {read}, {}", says_otherwise(HELD))
            } else {
                says_otherwise(&content)
            };
            lines.push(format!("    let handed: Bool = {handed}"));
            lines.push(format!("    if {differs} {{ {refuse} }}"));
        }
    }
    lines.push("}".to_string());
    lines
}

/// The local the walk of the chain `list` keeps the input `input` it hands each entry in, set from
/// the entry before (`ListWrite::previous`): zero for the first entry. A field is read through the
/// source and never bare, so the local hides no field.
fn previous_local(list: &str, input: &str) -> String {
    format!("{list}_{input}_previous")
}

/// The statements that write the entries of the list or chain `list`, which the encode reads as
/// `entries`, each a source of `child`'s codec, by its printed encode into bytes of the list's
/// own, appended to the frame once every entry is written: handed `arguments` after the sink,
/// each input the chain supplies set from the entry just written, and, on a chain that ends on
/// its entries' flag, the flag set through the entry codec's `Overrides` on every entry but the
/// last (docs/adr/0065 §6 R7), whatever flag the entry presents.
fn list_write(
    plan: &EncodePlan<'_>,
    list: &CodecField,
    walk: &ListWrite,
    child: &str,
    entries: &str,
    arguments: &str,
) -> Vec<String> {
    let id = &list.id;
    let written = format!("{id}_entries");
    let left = format!("{id}_left");
    let mut lines = vec![format!("var {written} = BytesSink()")];
    for (input, _) in &walk.previous {
        lines.push(format!("var {}: UInt8 = 0", previous_local(id, input)));
    }
    let overrides = match &walk.ends {
        Some((carrier, flag)) => {
            lines.push(format!("var {left} = {entries}.count"));
            format!(
                ", overrides: {child}.Overrides({}: {left} != 0)",
                override_member(carrier, flag)
            )
        }
        None => String::new(),
    };
    lines.push(format!("for entry in {entries} {{"));
    if walk.ends.is_some() {
        lines.push(format!("    {left} -= 1"));
    }
    lines.push(format!(
        "    try {child}.encodeSource(entry, into: &{written}{arguments}{overrides})"
    ));
    for (input, kind) in &walk.previous {
        lines.push(format!(
            "    {} = {} == 0x{kind:X} ? 1 : 0",
            previous_local(id, input),
            entry_identifier(plan, id, "entry")
        ));
    }
    lines.push("}".to_string());
    lines.push(format!("frame += {written}.bytes"));
    lines
}

/// The statements that write the chain `list` of a bare entry's place as slots (docs/adr/0065 §6
/// R6a), each entry a source of `child`'s codec, by its printed encode into bytes of the chain's
/// own, appended to the frame once every slot is written: the others, which the encode reads as
/// `entries`, in order, and `named`, the entry of the kind `ListWrite::slots` names, where the
/// value holds its place (`{list}_place`, `nil` where it holds none). Each is handed `arguments`
/// after the sink; an input the chain supplies from the entry before is set from the slot just
/// written, the named entry being of its kind; and on a chain that ends on its entries' flag the
/// flag is set on every slot but the last (§6 R7), whatever the entry presents.
fn slots_write(
    plan: &EncodePlan<'_>,
    list: &CodecField,
    walk: &ListWrite,
    child: &str,
    entries: &str,
    arguments: &str,
    named: &str,
) -> Vec<String> {
    let id = &list.id;
    let kind = walk
        .slots
        .expect("a chain walked as slots names its bare entry's kind");
    let written = format!("{id}_entries");
    let count = format!("{id}_slots");
    let next = format!("{id}_next");
    let at = format!("{id}_at");
    let place = place_local(id);
    let mut lines = vec![format!("var {written} = BytesSink()")];
    for (input, _) in &walk.previous {
        lines.push(format!("var {}: UInt8 = 0", previous_local(id, input)));
    }
    lines.push(format!(
        "let {count} = {entries}.count + ({place} != nil ? 1 : 0)"
    ));
    lines.push(format!("var {next} = 0"));
    let overrides = match &walk.ends {
        Some((carrier, flag)) => format!(
            ", overrides: {child}.Overrides({}: {at} + 1 < {count})",
            override_member(carrier, flag)
        ),
        None => String::new(),
    };
    lines.push(format!("for {at} in 0..<{count} {{"));
    lines.push(format!("    if {at} == {place} {{"));
    lines.push(format!(
        "        try {child}.encodeSource({named}, into: &{written}{arguments}{overrides})"
    ));
    for (input, waits) in &walk.previous {
        lines.push(format!(
            "        {} = {}",
            previous_local(id, input),
            u8::from(kind == *waits)
        ));
    }
    lines.push("    } else {".to_string());
    lines.push(format!("        let entry = {entries}[{next}]"));
    lines.push(format!("        {next} += 1"));
    lines.push(format!(
        "        try {child}.encodeSource(entry, into: &{written}{arguments}{overrides})"
    ));
    for (input, waits) in &walk.previous {
        lines.push(format!(
            "        {} = {} == 0x{waits:X} ? 1 : 0",
            previous_local(id, input),
            entry_identifier(plan, id, "entry")
        ));
    }
    lines.push("    }".to_string());
    lines.push("}".to_string());
    lines.push(format!("frame += {written}.bytes"));
    lines
}

/// One arm of a printed codec's variant, as the Swift printer spells it.
pub(super) struct ArmText {
    /// The case of the arms enum: the arm codec's alias in lower camel case, as the decode names
    /// it; unused for the catch-all, whose case is `default`.
    case: String,
    /// The Swift struct of the arm's codec.
    child: String,
    /// The associated type of the protocol the arm's body is a source of: the alias in upper camel
    /// case and `_Arm`, or `Default_Arm` for the catch-all. The `_` is in no codec's type name, so
    /// it hides none.
    assoc: String,
    /// The generic parameter of the arms enum the arm's body is: `A` and the arm's place, or `D`
    /// for the catch-all.
    generic: String,
    /// The condition a build leaves the arm out by, for an arm under a build feature
    /// (docs/adr/0072).
    off: Option<String>,
    /// The arm's tag, or `None` for the catch-all.
    tag: Option<u64>,
}

impl ArmText {
    /// The pattern that binds the body of this arm as `bound`.
    fn pattern(&self, bound: &str) -> String {
        match self.tag {
            Some(_) => format!(".{}(let {bound})", self.case),
            None => format!(".default(_, let {bound})"),
        }
    }

    /// The pattern that matches this arm and binds nothing.
    fn bare(&self) -> String {
        match self.tag {
            Some(_) => format!(".{}", self.case),
            None => ".default".to_string(),
        }
    }

    /// `lines` in the build that holds this arm: inside its condition where a feature decides it.
    fn guarded(&self, lines: Vec<String>) -> Vec<String> {
        match &self.off {
            None => lines,
            Some(off) => std::iter::once(format!("#if !{off}"))
                .chain(lines)
                .chain(std::iter::once("#endif".to_string()))
                .collect(),
        }
    }
}

/// The arms of the variant of `plan`, in the plan's order, the struct of each arm's codec in
/// `arm_types` at the same place.
pub(super) fn arm_texts(plan: &EncodePlan<'_>, arm_types: &[String]) -> Vec<ArmText> {
    plan.arms
        .iter()
        .zip(arm_types)
        .enumerate()
        .map(|(i, (arm, child))| {
            let known = arm.tag.is_some();
            ArmText {
                case: filters::to_camel_case(arm.codec.to_string()),
                child: child.clone(),
                assoc: if known {
                    format!("{}_Arm", filters::to_pascal_case(arm.codec.to_string()))
                } else {
                    "Default_Arm".to_string()
                },
                generic: if known {
                    format!("A{i}")
                } else {
                    "D".to_string()
                },
                off: arm.feature.map(super::feature_off_condition),
                tag: arm.tag,
            }
        })
        .collect()
}

/// The generic enum of the arms a printed codec named `struct_name` presents its variant as.
fn arms_enum(struct_name: &str) -> String {
    format!("{struct_name}Arms")
}

/// The member of a printed codec's `Overrides` that sets the flag `flag` of the carrier `carrier`:
/// the two names in lower camel case, so two carriers' flags of one name are two members.
pub(crate) fn override_member(carrier: &str, flag: &str) -> String {
    filters::to_camel_case(format!("{carrier}_{flag}"))
}

/// The local a printed encode folds the flags it derives or a parent sets into the carrier
/// `carrier` in. A field is read through the source and never bare, so the local hides no field.
fn folded_carrier(carrier: &str) -> String {
    format!("{carrier}_folded")
}
