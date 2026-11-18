// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The gate groups of a printed Kotlin codec (docs/adr/0065 §6 R4b, R4c).
//!
//! A group is the elements one gate, or a disjunction of gates, decides
//! together. A Kotlin value holds it as one property, named as every printer
//! names it ([`group_accessor`]): a sealed class of a data class for each arm
//! where the group has several, each holding the members present while the
//! gates hold the arm's values, or one data class where it has one; `null`
//! where the gates allow no arm. The encode derives every gate of the group
//! from the arm the value holds, and reads each member off that arm, `null`
//! where the arm does not hold it, so each statement that reads a member reads
//! it as it reads an element a gate decides.
//!
//! A group of a chain's memberships (docs/adr/0065 §6 R6b) holds the chain in
//! every arm, each in a class of the kinds the arm says it holds an entry of
//! and none of ([`chain_class`]), and the encode reads the one list of entries
//! every such class holds.

use super::super::rust_source::{
    chain_optional_in, group_accessor, group_arm_name, group_type, holds_chain,
};
use super::super::*;
use super::{input_name, CarrierBits, ListBound, Member, GATE_DISAGREES};
use crate::forge::codec_roles::{GateGroup, GateVar, GroupArm};
use crate::forge::encode_plan::{Bound, EncodePlan};
use std::collections::{BTreeMap, BTreeSet};

/// The class the Kotlin value of the codec whose data class is `struct_name`
/// holds `group` in, named as every printer names it ([`group_type`]): the
/// sealed class of its arms, or the data class of its one arm. The one place
/// the Kotlin name is made, which the printer declares and the conformance
/// harness makes a value of.
pub(crate) fn group_class(struct_name: &str, group: &GateGroup) -> String {
    group_type(struct_name, group)
}

/// The class of the arm `arm` of `group`, a group of the codec whose data
/// class is `struct_name`: nested in the group's sealed class and named after
/// the gates it holds ([`group_arm_name`]) where the group has several arms,
/// and the group's own class where it has one.
pub(crate) fn group_arm_class(struct_name: &str, group: &GateGroup, arm: &GroupArm) -> String {
    let class = group_class(struct_name, group);
    if group.is_sum() {
        format!("{class}.{}", group_arm_name(group, arm))
    } else {
        class
    }
}

/// The property a value holds `group` in.
pub(crate) fn group_property(group: &GateGroup) -> String {
    group_accessor(group)
}

/// A gate group as the value holds it: one property of the data class, which
/// the template declares and its decode makes.
pub(crate) struct GroupField {
    /// The property, the group's accessor.
    pub(crate) name: String,
    /// The type it is declared as: the group's class, `?` where the gates allow
    /// no arm.
    pub(crate) ty: String,
    /// The value it holds by default: its first arm made of each member's
    /// default, or for a group of a chain's memberships the arm where no gate
    /// holds, or `null` where the gates allow no arm.
    pub(crate) default: String,
    /// What the decode makes it of: the arm the gates it read select, made of
    /// the locals it read each member into, or `null` where they select none.
    pub(crate) made: String,
}

/// Whether the encode's local of the member `field` of `group` may be `null`:
/// an arm holds no `field`, the gates allow no arm, or `field` is the chain
/// whose memberships are the group's gates and an arm may hold none of it
/// (docs/adr/0065 §6 R6b).
pub(crate) fn member_optional(group: &GateGroup, field: &CodecField) -> bool {
    let chain = holds_chain(group, field);
    group.absent.is_some()
        || group.arms.iter().any(|arm| {
            !arm.fields.contains(&field.id) || (chain && chain_optional_in(group, field, arm))
        })
}

/// The chain whose memberships are gates of a group (docs/adr/0065 §6 R6b),
/// which every arm of the group holds, each in the class of the kinds it says.
pub(super) struct HeldChain<'f> {
    /// The chain.
    pub(super) field: &'f CodecField,
    /// The data class of the entries' codec, whose source interface is
    /// `{entry}Source`.
    pub(super) entry: String,
    /// How many entries the chain holds.
    pub(super) bound: Bound,
    /// The kind of an entry `entry`, the identifier the chain tells its entries
    /// apart by, as a `ULong` (`kotlin_source::entry_identifier`).
    pub(super) kind: String,
    /// Where the group's one gate is a membership of a bare entry (docs/adr/0065
    /// §6 R6a), the entry of that kind, whose identifier is all of its value,
    /// as an expression of the entries' codec's data class.
    pub(super) named: Option<String>,
}

/// The class the Kotlin value of the codec whose data class is `struct_name`
/// holds its chain `chain` in, in the arm of a group of its one membership of
/// a bare entry of the kind `key` where the gate holds (docs/adr/0065 §6
/// R6a): the other entries and the place of the named entry among them. The
/// one place the name is made, as [`chain_class`] is for a chain held whole.
pub(crate) fn placed_class(struct_name: &str, chain: &str, key: u64) -> String {
    format!(
        "{struct_name}{}Has{key:X}Placed",
        filters::to_pascal_case(chain.to_string())
    )
}

/// The class `class` the chain `chain` the arm where its bare entry's gate
/// holds is held in (docs/adr/0065 §6 R6a): the entries other than the named
/// one and the place it stands at among them, as the Rust and C++ runtimes'
/// `Placed` holds them. `of` makes a value only of others that make, with the
/// named entry, a chain within the bound, a place at most past the last of
/// them, and others before the place none of the kind, so the named entry is
/// the first of its kind, which is the entry a decode takes out of the chain
/// it read (`placing`). `named` is the entry of the kind, which the encode
/// writes at the place.
fn placed_declaration(struct_name: &str, chain: &HeldChain<'_>, class: &str, key: u64) -> String {
    let id = &chain.field.id;
    let bound = ListBound::of(&chain.bound);
    let source = format!("{}Source", chain.entry);
    let named = chain
        .named
        .as_deref()
        .expect("a chain of a bare entry's group names the entry of its kind");
    // The chain the others make with the named entry holds one more than they
    // do, so it is never below a least of one.
    let mut tests: Vec<String> = Vec::new();
    if bound.least > 1 {
        tests.push("rest.size + 1 >= LEAST".to_string());
    }
    if chain.bound.most.is_some() {
        tests.push("rest.size < MOST".to_string());
    }
    tests.push("place in 0..rest.size".to_string());
    tests.push(format!(
        "rest.take(place).none {{ kind(it) == 0x{key:X}uL }}"
    ));
    format!(
        "/// The `{id}` chain of [{struct_name}] where it holds an entry of the kind 0x{key:X},\n\
         /// which is bare, its identifier all it is (docs/adr/0065 §6 R6a, docs/adr/0081 §2):\n\
         /// the other entries and the place the named entry stands at among them, in a\n\
         /// chain of {held}. Its constructor is private and [of] makes a value only of\n\
         /// others that make such a chain with the named entry, which is then the first\n\
         /// of its kind.\n\
         class {class} private constructor(\n\
         \x20   /// The entries other than the named one, in order.\n\
         \x20   val rest: List<{source}>,\n\
         \x20   /// Where the named entry stands among them.\n\
         \x20   val place: Int,\n\
         ) {{\n\
         \x20   override fun equals(other: Any?): Boolean =\n\
         \x20       other is {class} && other.rest == rest && other.place == place\n\
         \n\
         \x20   override fun hashCode(): Int = 31 * rest.hashCode() + place\n\
         \n\
         \x20   override fun toString(): String = \"{class}(rest=$rest, place=$place)\"\n\
         \n\
         \x20   companion object {{{consts}\n\
         \x20       /// The kind of `entry`: the identifier the chain tells its entries apart by.\n\
         \x20       private fun kind(entry: {source}): ULong = {identifier}\n\
         \n\
         \x20       /// `rest` with the named entry at `place` among them, or `null` where the\n\
         \x20       /// chain they make lies outside its bound, the place is past the last of\n\
         \x20       /// them, or one before the place is of the kind.\n\
         \x20       fun of(rest: List<{source}>, place: Int): {class}? =\n\
         \x20           if ({tests}) {class}(rest.toList(), place) else null\n\
         \n\
         \x20       /// `entries`, a chain read whole, with its first entry of the kind taken\n\
         \x20       /// out at its place, or `null` where it holds none of the kind or lies\n\
         \x20       /// outside the bound.\n\
         \x20       fun placing(entries: List<{source}>): {class}? {{\n\
         \x20           val place = entries.indexOfFirst {{ kind(it) == 0x{key:X}uL }}\n\
         \x20           if (place < 0) return null\n\
         \x20           return of(entries.filterIndexed {{ i, _ -> i != place }}, place)\n\
         \x20       }}\n\
         \n\
         \x20       /// The entry of the kind: its identifier, and nothing else to say.\n\
         \x20       fun named(): {entry} = {named}\n\
         \x20   }}\n\
         }}\n\n",
        held = bound.held,
        consts = bound.consts,
        identifier = chain.kind,
        tests = tests.join(" && "),
        entry = chain.entry,
    )
}

/// The class the Kotlin value of the codec whose data class is `struct_name`
/// holds its chain `chain` in, in an arm of a group of its memberships that
/// says the chain holds an entry of each kind of `holds` and of no kind of
/// `lacks` (docs/adr/0065 §6 R6b): the codec, the chain, each kind after `Has`
/// or `Lacks`, then `Chain`, as a list's class ends in `List`. Arms that say
/// the same of the chain hold it in one class. The one place the name is made:
/// the printer declares the class by it and the conformance harness makes a
/// value of it.
pub(crate) fn chain_class(struct_name: &str, chain: &str, holds: &[u64], lacks: &[u64]) -> String {
    let kinds = |word: &str, keys: &[u64]| -> String {
        keys.iter().map(|key| format!("{word}{key:X}")).collect()
    };
    format!(
        "{struct_name}{}{}{}Chain",
        filters::to_pascal_case(chain.to_string()),
        kinds("Has", holds),
        kinds("Lacks", lacks)
    )
}

/// The value class `class` the chain `chain` an arm holds is held in: a
/// private constructor over the entries, and `of`, which makes a value only of
/// entries within the chain's bound that hold an entry of each kind of `holds`
/// and of no kind of `lacks`, each told apart by the chain's `entry-id`, copied
/// into a list of its own. Where the arm says the chain holds no kind, a value
/// made by default holds the fewest entries the bound holds, each made by
/// default, which hold no kind it lacks, as the C++ `Chain` asserts of its own.
fn chain_declaration(
    struct_name: &str,
    chain: &HeldChain<'_>,
    class: &str,
    holds: &[u64],
    lacks: &[u64],
) -> String {
    let id = &chain.field.id;
    let bound = ListBound::of(&chain.bound);
    let source = format!("{}Source", chain.entry);
    let kinds = |keys: &[u64]| -> String {
        keys.iter()
            .map(|key| format!("0x{key:X}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut said = Vec::new();
    if !holds.is_empty() {
        said.push(format!("an entry of each kind of {}", kinds(holds)));
    }
    if !lacks.is_empty() {
        said.push(format!("no entry of a kind of {}", kinds(lacks)));
    }
    let mut tests: Vec<String> = bound.test.iter().cloned().collect();
    tests.extend(
        holds
            .iter()
            .map(|key| format!("entries.any {{ kind(it) == 0x{key:X}uL }}")),
    );
    tests.extend(
        lacks
            .iter()
            .map(|key| format!("entries.none {{ kind(it) == 0x{key:X}uL }}")),
    );
    let fewest = if holds.is_empty() {
        format!(
            "\n\n\
             \x20       /// {}, which hold no kind the chain\n\
             \x20       /// lacks: what a value made by default holds.\n\
             \x20       fun fewest(): {class} =\n\
             \x20           checkNotNull(of({})) {{ \"a value-made entry is of no kind the chain lacks\" }}",
            bound.fewest_words(),
            bound.fewest(&chain.entry)
        )
    } else {
        String::new()
    };
    format!(
        "/// The `{id}` chain of [{struct_name}] where it holds {said}, {held}, held\n\
         /// to that where a value of it is made (docs/adr/0065 §6 R6b, docs/adr/0081 §2):\n\
         /// its constructor is private and [of] makes a value only of entries within the\n\
         /// bound that hold those kinds, which it copies, so no list it was made of can\n\
         /// change it after.\n\
         @JvmInline\n\
         value class {class} private constructor(val entries: List<{source}>) : \
         com.sce.forge.runtime.Counted {{\n\
         \x20   /// How many entries the chain holds.\n\
         \x20   override val size: Int\n\
         \x20       get() = entries.size\n\
         \n\
         \x20   companion object {{{consts}\n\
         \x20       /// The kind of `entry`: the identifier the chain tells its entries apart by.\n\
         \x20       private fun kind(entry: {source}): ULong = {identifier}\n\
         \n\
         \x20       /// `entries` as a value of the chain, or `null` where they lie outside its\n\
         \x20       /// bound, lack a kind it holds or hold one it lacks.\n\
         \x20       fun of(entries: List<{source}>): {class}? =\n\
         \x20           if ({tests}) {class}(entries.toList()) else null{fewest}\n\
         \x20   }}\n\
         }}\n\n",
        said = said.join(" and "),
        held = bound.held,
        consts = bound.consts,
        identifier = chain.kind,
        tests = tests.join(" && "),
    )
}

/// The classes of `group` into `before`, and the property the value of the
/// codec `plan`, whose data class is `struct_name`, holds it in. Each member is
/// held as `members` says, always there in the arm that holds it, but for the
/// chain whose memberships are gates of the group (docs/adr/0065 §6 R6b),
/// which each arm holds in the class of the kinds it says of it, `null` where
/// it is gated and the arm says it holds no kind.
pub(super) fn declare(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    struct_name: &str,
    group: &GateGroup,
    members: &BTreeMap<String, Member>,
    chain: Option<&HeldChain<'_>>,
    before: &mut String,
) -> GroupField {
    let name = group_property(group);
    let class = group_class(struct_name, group);
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
    // The arm a value made by default holds: the arm present while no gate
    // holds, where there is one, else the first, as in Rust, C++ and Python. In
    // a group of a chain's memberships an arm that says the chain holds a kind
    // has no chain made by default, and the arm where no gate holds says none.
    let default_at = group
        .arms
        .iter()
        .position(|arm| arm.values.iter().all(|value| !value))
        .unwrap_or(0);
    // Each arm, its members as the constructor's parameters, and how a decode
    // makes it of what it read.
    let mut arms = String::new();
    let mut made: Vec<(String, String)> = Vec::new();
    let mut first_default = String::new();
    // The classes of the chain each arm holds, each declared once.
    let mut chains: BTreeSet<String> = BTreeSet::new();
    for (i, arm) in group.arms.iter().enumerate() {
        let arm_class = group_arm_class(struct_name, group, arm);
        let mut params = Vec::new();
        let mut args = Vec::new();
        for id in &arm.fields {
            // The chain, in the class of the kinds the arm says of it.
            if let Some(chain) = chain.filter(|chain| chain.field.id == *id) {
                let (holds, lacks) = group.kinds_in(arm);
                let property = l.codec_field_id(id);
                let local = l.codec_field_decode_local(id);
                // Where the one gate on a bare entry holds, the others and
                // the named entry's place (docs/adr/0065 §6 R6a); the decode
                // read the chain, as the gate says, and takes the first entry
                // of the kind out of it.
                if placed_in(group, arm) {
                    let key = holds[0];
                    let placed = placed_class(struct_name, id, key);
                    if chains.insert(placed.clone()) {
                        before.push_str(&placed_declaration(struct_name, chain, &placed, key));
                    }
                    let read = if chain.field.present_if.is_some() {
                        format!("{local}!!")
                    } else {
                        local
                    };
                    // A chain the gate says holds the bare entry, read without
                    // one of its kind, says two things of one frame.
                    params.push(format!("val {property}: {placed}"));
                    args.push(format!(
                        "{property} = {placed}.placing({read}) ?: {GATE_DISAGREES}"
                    ));
                    continue;
                }
                let kinds = chain_class(struct_name, id, &holds, &lacks);
                if chains.insert(kinds.clone()) {
                    before.push_str(&chain_declaration(
                        struct_name,
                        chain,
                        &kinds,
                        &holds,
                        &lacks,
                    ));
                }
                // A chain read that holds a kind the arm's gates say it lacks,
                // or lacks one they say it holds, is refused as a gate the
                // frame gave where its predicate does not hold.
                if chain_optional_in(group, chain.field, arm) {
                    // Gated, and the arm says it holds no kind: absent by
                    // default, and absent where the decode read none.
                    params.push(format!("val {property}: {kinds}? = null"));
                    args.push(format!(
                        "{property} = {local}?.let {{ {kinds}.of(it) ?: {GATE_DISAGREES} }}"
                    ));
                } else {
                    // Where the arm says it holds a kind, the decode read it,
                    // as the gate that selects the arm says.
                    let read = if chain.field.present_if.is_some() {
                        format!("{local}!!")
                    } else {
                        local
                    };
                    params.push(if holds.is_empty() {
                        format!("val {property}: {kinds} = {kinds}.fewest()")
                    } else {
                        format!("val {property}: {kinds}")
                    });
                    args.push(format!(
                        "{property} = {kinds}.of({read}) ?: {GATE_DISAGREES}"
                    ));
                }
                continue;
            }
            // Two lists of one count are one member of an arm that holds both
            // (docs/adr/0065 §6 R5), where the first is: an `Aligned` of the
            // two, each in the class of its bound, made only where they hold
            // as many entries as each other.
            if group.is_second_in(arm, id) {
                continue;
            }
            let member = &members[id.as_str()];
            if let Some(second) = group.second_of(arm, id) {
                let paired = &members[second];
                let property = aligned_property(l, id, second);
                params.push(format!(
                    "val {property}: {ALIGNED}<{}, {}> = {ALIGNED}.of({}, {})!!",
                    member.ty, paired.ty, member.default, paired.default
                ));
                // The decode read both lists under the one count the frame
                // wrote, so they hold as many entries as each other and the
                // factory answers a value for every frame that reaches it.
                args.push(format!(
                    "{property} = {ALIGNED}.of({}, {}) ?: \
                     error(\"two lists read under one count hold as many entries\")",
                    member.made, paired.made
                ));
                continue;
            }
            let property = l.codec_field_id(id);
            params.push(format!(
                "val {property}: {} = {}",
                member.ty, member.default
            ));
            args.push(format!("{property} = {}", member.made));
        }
        // An arm of no member is one value, an object.
        let simple = if group.is_sum() {
            group_arm_name(group, arm)
        } else {
            class.clone()
        };
        let (declared, value) = if params.is_empty() {
            (format!("object {simple}"), arm_class.clone())
        } else {
            (
                format!("data class {simple}(\n{}\n)", indented_params(&params)),
                format!("{arm_class}({})", args.join(", ")),
            )
        };
        if i == default_at {
            first_default = if params.is_empty() {
                arm_class.clone()
            } else {
                format!("{arm_class}()")
            };
        }
        let doc = format!(
            "/// The `{name}` group of [{struct_name}] where {} (docs/adr/0065 §6 R4): the\n\
             /// members present then.\n",
            when(arm)
        );
        if group.is_sum() {
            arms.push_str(&indent_lines(&format!("{doc}{declared} : {class}()\n")));
        } else {
            before.push_str(&format!("{doc}{declared}\n\n"));
        }
        made.push((value, decoded_test(plan, l, group, &arm.values)));
    }
    if group.is_sum() {
        before.push_str(&format!(
            "/// The `{name}` group of [{struct_name}]: the arm the value holds, each the\n\
             /// members present while the gates hold its values (docs/adr/0065 §6 R4).\n\
             sealed class {class} {{\n{arms}}}\n\n"
        ));
    }
    // The arms the gates select, in turn; where the gates may select none, the
    // value holds no arm, and the last arm is otherwise the one held.
    let mut decoded = String::new();
    let last = made.len() - 1;
    for (i, (arm, test)) in made.into_iter().enumerate() {
        if i == last && group.absent.is_none() {
            decoded.push_str(&arm);
        } else {
            decoded.push_str(&format!("if ({test}) {arm} else "));
        }
    }
    let (ty, default) = match &group.absent {
        Some(_) => {
            decoded.push_str("null");
            (format!("{class}?"), "null".to_string())
        }
        None => (class, first_default),
    };
    GroupField {
        name,
        ty,
        default,
        made: decoded,
    }
}

/// `params`, the parameters of a constructor, one a line at the indentation of
/// a class's body, separated by commas.
fn indented_params(params: &[String]) -> String {
    params
        .iter()
        .map(|p| format!("    {p}"))
        .collect::<Vec<_>>()
        .join(",\n")
}

/// Each line of `text` one level deeper.
fn indent_lines(text: &str) -> String {
    text.lines()
        .map(|line| {
            if line.is_empty() {
                "\n".to_string()
            } else {
                format!("    {line}\n")
            }
        })
        .collect()
}

/// Whether the gates of `group`, as the decode of the codec `plan` read them,
/// hold `values`: each flag tested on the local its carrier was read into, each
/// input as the decode is handed it.
fn decoded_test(plan: &EncodePlan<'_>, l: &LangCtx, group: &GateGroup, values: &[bool]) -> String {
    group
        .gates
        .iter()
        .zip(values)
        .map(|(gate, value)| {
            let compare = if *value { "!=" } else { "==" };
            match gate {
                GateVar::Flag { carrier, flag } => {
                    let field = plan
                        .model
                        .fields
                        .iter()
                        .find(|f| &f.id == carrier)
                        .expect("a gate's carrier is a field of the codec");
                    let bit = field
                        .flags
                        .iter()
                        .find(|d| &d.name == flag)
                        .map(|d| d.bit)
                        .expect("the parser found the flag a gate reads");
                    let bits = CarrierBits::of(&field.sce_type);
                    format!(
                        "(({} shr {bit}) and {}) {compare} {}",
                        bits.viewed(&l.codec_field_decode_local(carrier)),
                        bits.literal(1),
                        bits.literal(0)
                    )
                }
                GateVar::Input(name) => format!("{}.toUInt() {compare} 0u", input_name(name)),
                // Whether the chain the decode read holds an entry of the
                // kind: the local the decode declares after the chain for
                // each kind a gate asks of it, read off every entry it read.
                GateVar::Member { chain, key, .. } => {
                    let has = chain_has_local(chain, *key, crate::generator::Language::Kotlin);
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

/// Whether the gate at `gate` among those of `group`, a group of the codec
/// whose data class is `struct_name`, holds where the value holds `held`, the
/// group as it holds it: it holds one of the arms that set the gate, or no arm
/// where the gates the value then has set it.
pub(crate) fn gate_holds(struct_name: &str, group: &GateGroup, gate: usize, held: &str) -> String {
    let mut tests: Vec<String> = Vec::new();
    for arm in group.arms.iter().filter(|arm| arm.values[gate]) {
        tests.push(if group.is_sum() {
            format!("{held} is {}", group_arm_class(struct_name, group, arm))
        } else {
            format!("{held} != null")
        });
    }
    if group.absent.as_ref().is_some_and(|absent| absent[gate]) {
        tests.push(format!("{held} == null"));
    }
    match tests.as_slice() {
        [] => "false".to_string(),
        [one] => one.clone(),
        many => format!("({})", many.join(" || ")),
    }
}

/// The expression the encode binds the member `field` of `group`, a group of
/// the codec whose data class is `struct_name`, to, off `held`, the group as
/// the value holds it: what the arm the value holds holds of it, `null` where
/// that arm holds none or the value holds no arm. Each arm is its own case, as
/// a case of several classes narrows to none of them. The chain whose
/// memberships are gates of the group is read as the list of its entries,
/// which every arm's class of it holds (docs/adr/0065 §6 R6b).
pub(crate) fn member_read(
    l: &LangCtx,
    struct_name: &str,
    group: &GateGroup,
    field: &CodecField,
    held: &str,
) -> String {
    let id = field.id.as_str();
    let property = l.codec_field_id(id);
    let chain = holds_chain(group, field);
    if !group.is_sum() {
        let entries = if chain { ".entries" } else { "" };
        return if group.absent.is_some() {
            format!("{held}?.{property}{entries}")
        } else {
            format!("{held}.{property}{entries}")
        };
    }
    // What an arm that holds the member reads it by: its property, or one of
    // the two lists of one count it holds as one `Aligned` (docs/adr/0065 §6
    // R5), or the entries of the chain it holds, where it may hold none.
    let read = |arm: &GroupArm| -> String {
        if chain {
            // The others of a bare entry's place (docs/adr/0065 §6 R6a), whose
            // named entry the walk writes at the place.
            if placed_in(group, arm) {
                format!("_g.{property}.rest")
            } else if chain_optional_in(group, field, arm) {
                format!("_g.{property}?.entries")
            } else {
                format!("_g.{property}.entries")
            }
        } else if let Some(second) = group.second_of(arm, id) {
            format!("_g.{}.first", aligned_property(l, id, second))
        } else if group.is_second_in(arm, id) {
            let (first, _) = group
                .aligned
                .as_ref()
                .expect("the second of two lists of one count is aligned");
            format!("_g.{}.second", aligned_property(l, first, id))
        } else {
            format!("_g.{property}")
        }
    };
    let mut cases: Vec<String> = group
        .arms
        .iter()
        .filter(|arm| arm.fields.iter().any(|f| f == id))
        .map(|arm| {
            format!(
                "is {} -> {}",
                group_arm_class(struct_name, group, arm),
                read(arm)
            )
        })
        .collect();
    // No member where the value holds no arm, or an arm that holds none. The
    // chain every arm holds, so an arm that may hold none of it reads `null`
    // off its own case.
    if group.absent.is_some()
        || group
            .arms
            .iter()
            .any(|arm| !arm.fields.iter().any(|f| f == id))
    {
        cases.push("else -> null".to_string());
    }
    format!("when (val _g = {held}) {{ {} }}", cases.join("; "))
}

/// Whether `arm` of `group` holds the chain of its one gate on a bare entry as
/// the others and the named entry's place (docs/adr/0065 §6 R6a): the gate
/// holds in it.
pub(crate) fn placed_in(group: &GateGroup, arm: &GroupArm) -> bool {
    group.member_is_bare() && !group.kinds_in(arm).0.is_empty()
}

/// The expression the encode binds the place of the named entry of `field`,
/// the chain of the bare entry's group `group` of the codec whose data class
/// is `struct_name` (docs/adr/0065 §6 R6a), to, off `held`, the group as the
/// value holds it: the place the arm where the gate holds holds, and `null`
/// from any other arm, which holds no named entry.
pub(crate) fn place_read(
    l: &LangCtx,
    struct_name: &str,
    group: &GateGroup,
    field: &CodecField,
    held: &str,
) -> String {
    let property = l.codec_field_id(&field.id);
    let cases: Vec<String> = group
        .arms
        .iter()
        .filter(|arm| placed_in(group, arm))
        .map(|arm| {
            format!(
                "is {} -> _g.{property}.place",
                group_arm_class(struct_name, group, arm)
            )
        })
        .collect();
    format!(
        "when (val _g = {held}) {{ {}; else -> null }}",
        cases.join("; ")
    )
}

/// The runtime's class of two lists of one count (docs/adr/0065 §6 R5),
/// written whole so the codec's file imports nothing for it.
pub(crate) const ALIGNED: &str = "com.sce.forge.runtime.Aligned";

/// The property an arm holds the two lists `first` and `second` of one count
/// in, as one [`ALIGNED`], named as every printer names it (`aligned_name`).
pub(crate) fn aligned_property(l: &LangCtx, first: &str, second: &str) -> String {
    super::super::rust_source::aligned_name(&l.codec_field_id(first), &l.codec_field_id(second))
}
