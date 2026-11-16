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

use super::super::rust_source::{group_accessor, group_arm_name, group_type};
use super::super::*;
use super::{input_name, CarrierBits, Member};
use crate::forge::codec_roles::{GateGroup, GateVar, GroupArm};
use crate::forge::encode_plan::EncodePlan;
use std::collections::BTreeMap;

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
    /// default, or `null` where the gates allow no arm.
    pub(crate) default: String,
    /// What the decode makes it of: the arm the gates it read select, made of
    /// the locals it read each member into, or `null` where they select none.
    pub(crate) made: String,
}

/// Whether the encode's local of the member `id` of `group` may be `null`: an
/// arm holds no `id`, or the gates allow no arm.
pub(crate) fn member_optional(group: &GateGroup, id: &str) -> bool {
    group.absent.is_some()
        || group
            .arms
            .iter()
            .any(|arm| !arm.fields.iter().any(|f| f == id))
}

/// The classes of `group` into `before`, and the property the value of the
/// codec `plan`, whose data class is `struct_name`, holds it in. Each member is
/// held as `members` says, always there in the arm that holds it.
pub(super) fn declare(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    struct_name: &str,
    group: &GateGroup,
    members: &BTreeMap<String, Member>,
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
    // Each arm, its members as the constructor's parameters, and how a decode
    // makes it of what it read.
    let mut arms = String::new();
    let mut made: Vec<(String, String)> = Vec::new();
    let mut first_default = String::new();
    for (i, arm) in group.arms.iter().enumerate() {
        let arm_class = group_arm_class(struct_name, group, arm);
        let mut params = Vec::new();
        let mut args = Vec::new();
        let mut defaults = Vec::new();
        for id in &arm.fields {
            let member = &members[id.as_str()];
            let property = l.codec_field_id(id);
            params.push(format!(
                "val {property}: {} = {}",
                member.ty, member.default
            ));
            args.push(format!("{property} = {}", member.made));
            defaults.push(member.default.clone());
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
        if i == 0 {
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
                GateVar::Member { .. } => {
                    unreachable!("`unlowered` prints no group of a chain's memberships")
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

/// The expression the encode binds the member `id` of `group`, a group of the
/// codec whose data class is `struct_name`, to, off `held`, the group as the
/// value holds it: what the arm the value holds holds of it, `null` where that
/// arm holds none or the value holds no arm. Each arm is its own case, as a
/// case of several classes narrows to none of them.
pub(crate) fn member_read(
    l: &LangCtx,
    struct_name: &str,
    group: &GateGroup,
    id: &str,
    held: &str,
) -> String {
    let property = l.codec_field_id(id);
    if !group.is_sum() {
        return if group.absent.is_some() {
            format!("{held}?.{property}")
        } else {
            format!("{held}.{property}")
        };
    }
    let mut cases: Vec<String> = group
        .arms
        .iter()
        .filter(|arm| arm.fields.iter().any(|f| f == id))
        .map(|arm| {
            format!(
                "is {} -> _g.{property}",
                group_arm_class(struct_name, group, arm)
            )
        })
        .collect();
    if member_optional(group, id) {
        cases.push("else -> null".to_string());
    }
    format!("when (val _g = {held}) {{ {} }}", cases.join("; "))
}
