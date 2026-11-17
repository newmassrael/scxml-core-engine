// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The gate groups of a printed C11 codec (docs/adr/0065 §6 R4b, R4c).
//!
//! A group is the elements one gate, or a disjunction of gates, decides
//! together. A C11 value holds it as one member, named as every printer names
//! it (`group_accessor`): a tagged struct, `{codec}_{group}_t`, of the kind of
//! the arm it holds and a union of a struct for each arm, each holding the
//! members present while the gates hold the arm's values, or the one arm's
//! struct where the group has one; the optional value of it where the gates
//! allow no arm. It is the shape the value already holds its variant in, and a
//! value holds no member of the group but in the arm that holds it, so no
//! value says a member is there that its gates leave out. A source presents
//! the group as the value holds it. The encode derives every gate of the group
//! from the arm the source presents, and binds each member to the optional
//! value an element a gate decides is read into, absent where the arm holds
//! none, so each statement that reads a member reads it as it reads a gated
//! element. The decode reads each member into a local of its own, as it reads
//! a number the encode derives, and makes the arm the gates it read select.

use super::super::rust_source::{group_accessor, group_arm_name};
use super::super::*;
use super::{derived_local, input_name};
use crate::forge::codec_roles::{GateGroup, GateVar};
use crate::forge::encode_plan::EncodePlan;

/// A gate group of a codec as the C11 printer names it.
pub(super) struct Group<'g> {
    /// The group.
    pub(super) group: &'g GateGroup,
    /// The member of the value's struct that holds it, the function of a
    /// source's table that presents it, and the stem of the encode's local
    /// it is read into.
    pub(super) name: String,
    /// Its tagged struct, or its one arm's struct.
    ty: String,
    /// The type a value holds it as: [`Self::ty`], or its optional value where
    /// the gates allow no arm.
    pub(super) held: String,
    /// Each arm, in the group's order.
    arms: Vec<ArmName>,
}

/// One arm of a [`Group`].
struct ArmName {
    /// Its member of the tagged struct's union.
    member: String,
    /// Its constant of the group's kind type.
    kind: String,
    /// Its struct.
    ty: String,
}

/// The groups of `plan`, a codec whose symbols are named after `snake`.
pub(super) fn groups<'p>(plan: &'p EncodePlan<'_>, snake: &str) -> Vec<Group<'p>> {
    plan.groups().iter().map(|g| Group::of(snake, g)).collect()
}

impl<'g> Group<'g> {
    /// `group`, a group of the codec `snake`.
    fn of(snake: &str, group: &'g GateGroup) -> Self {
        let name = group_accessor(group);
        let ty = format!("{snake}_{name}_t");
        let held = if group.absent.is_some() {
            format!("{snake}_{name}_optional_t")
        } else {
            ty.clone()
        };
        let arms = group
            .arms
            .iter()
            .map(|arm| {
                let member = filters::to_snake_case(group_arm_name(group, arm));
                ArmName {
                    kind: format!("{snake}_{name}_{member}").to_uppercase(),
                    ty: if group.is_sum() {
                        format!("{snake}_{name}_{member}_t")
                    } else {
                        ty.clone()
                    },
                    member,
                }
            })
            .collect();
        Group {
            group,
            name,
            ty,
            held,
            arms,
        }
    }

    /// The group's struct inside `held`, an expression of [`Self::held`]: the
    /// value of the optional one, where the gates allow no arm.
    fn base(&self, held: &str) -> String {
        if self.group.absent.is_some() {
            format!("{held}.value")
        } else {
            held.to_string()
        }
    }

    /// Whether `held`, an expression of [`Self::held`], holds arm `i`, or
    /// `None` where it holds that arm in every value.
    fn arm_test(&self, i: usize, held: &str) -> Option<String> {
        let base = self.base(held);
        let kind = self
            .group
            .is_sum()
            .then(|| format!("{base}.kind == {}", self.arms[i].kind));
        match (self.group.absent.is_some(), kind) {
            (true, Some(kind)) => Some(format!("{held}.present && {kind}")),
            (true, None) => Some(format!("{held}.present")),
            (false, kind) => kind,
        }
    }

    /// The member `id` of arm `i` inside `held`, an expression of
    /// [`Self::held`] that holds that arm.
    fn member_of(&self, i: usize, id: &str, held: &str) -> String {
        let base = self.base(held);
        let id = filters::to_snake_case(id.to_string());
        if self.group.is_sum() {
            format!("{base}.arm.{}.{id}", self.arms[i].member)
        } else {
            format!("{base}.{id}")
        }
    }

    /// The typedefs of the group, before the value's struct: the kind of its
    /// arms, a struct for each arm, the tagged struct, and the optional value
    /// of it where the gates allow no arm. Each member is held as its type, a
    /// number or an enum, always there in the arm that holds it.
    pub(super) fn declare(&self, plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
        let (name, ty) = (&self.name, &self.ty);
        let fields = |i: usize| -> String {
            let arm = &self.group.arms[i];
            if arm.fields.is_empty() {
                return "    /* A C struct declares at least one member; the arm holds\n     \
                        * none. */\n    char _reserved;\n"
                    .to_string();
            }
            arm.fields
                .iter()
                .map(|id| {
                    let field = plan
                        .model
                        .fields
                        .iter()
                        .find(|f| &f.id == id)
                        .expect("a member of a group is a field of the codec");
                    format!(
                        "    {} {};\n",
                        l.type_name(&field.sce_type),
                        filters::to_snake_case(id.clone())
                    )
                })
                .collect()
        };
        let when = |i: usize| -> String {
            self.group
                .gates
                .iter()
                .zip(&self.group.arms[i].values)
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
        let mut out = String::new();
        if self.group.is_sum() {
            let kinds: String = self
                .arms
                .iter()
                .map(|arm| format!("    {},\n", arm.kind))
                .collect();
            out.push_str(&format!(
                "/* The arms of the `{name}` group of {snake}_t (docs/adr/0065 §6 R4). */\n\
                 typedef enum {{\n{kinds}}} {snake}_{name}_kind_t;\n\n"
            ));
            let mut union = String::new();
            for (i, arm) in self.arms.iter().enumerate() {
                out.push_str(&format!(
                    "/* The `{name}` group of {snake}_t where {}: the members present\n \
                     * then. */\n\
                     typedef struct {{\n{}}} {};\n\n",
                    when(i),
                    fields(i),
                    arm.ty
                ));
                union.push_str(&format!("        {} {};\n", arm.ty, arm.member));
            }
            out.push_str(&format!(
                "/* The `{name}` group of {snake}_t: the arm the value holds, which `kind`\n \
                 * names, each the members present while the gates hold its values\n \
                 * (docs/adr/0065 §6 R4). */\n\
                 typedef struct {{\n    {snake}_{name}_kind_t kind;\n    union {{\n{union}    \
                 }} arm;\n}} {ty};\n\n"
            ));
        } else {
            out.push_str(&format!(
                "/* The `{name}` group of {snake}_t where {}: the members present then\n \
                 * (docs/adr/0065 §6 R4). */\n\
                 typedef struct {{\n{}}} {ty};\n\n",
                when(0),
                fields(0)
            ));
        }
        if self.group.absent.is_some() {
            out.push_str(&format!(
                "/* The `{name}` group of {snake}_t, absent where the gates allow no arm. */\n\
                 SCE_OPTIONAL_DECLARE_NAMED({snake}_{name}_optional, {ty})\n\n"
            ));
        }
        out
    }

    /// What the encode binds the member `id`, of the type `ty`, to, off
    /// `held`, the group as a source presents it: the optional value an
    /// element a gate decides is read into, present where the arm `held`
    /// holds holds `id` and absent where it holds none.
    pub(super) fn member_read(&self, l: &LangCtx, id: &str, ty: &SceType, held: &str) -> String {
        let optional = l.c11_codec_optional(ty, None);
        let mut read = optional.none();
        for (i, arm) in self.group.arms.iter().enumerate().rev() {
            if !arm.fields.iter().any(|f| f == id) {
                continue;
            }
            let some = optional.some(&self.member_of(i, id, held));
            read = match self.arm_test(i, held) {
                Some(test) => format!("({test} ? {some} : {read})"),
                None => some,
            };
        }
        read
    }

    /// Whether the gate at `gate` among the group's holds where a source
    /// presents `held`, the group as it holds it: the value holds an arm that
    /// sets it, or no arm where the gates the value then has set it. A C
    /// expression of `bool`'s value.
    pub(super) fn gate_value(&self, gate: usize, held: &str) -> String {
        let mut tests: Vec<String> = Vec::new();
        for (i, arm) in self.group.arms.iter().enumerate() {
            if !arm.values[gate] {
                continue;
            }
            tests.push(self.arm_test(i, held).unwrap_or_else(|| "1".to_string()));
        }
        if self
            .group
            .absent
            .as_ref()
            .is_some_and(|absent| absent[gate])
        {
            tests.push(format!("!{held}.present"));
        }
        if tests.is_empty() {
            "0".to_string()
        } else {
            format!("({})", tests.join(" || "))
        }
    }

    /// The statements that make the value's group of the locals the decode of
    /// the codec `plan` read its members into ([`derived_local`]), once every
    /// field is read: the arm the gates the decode read select, each flag tested
    /// on the carrier the decode read it into and each input as the decode is
    /// handed it, and no arm where they select none.
    pub(super) fn decoded(&self, plan: &EncodePlan<'_>) -> String {
        let name = &self.name;
        let held = format!("out->{name}");
        let base = self.base(&held);
        let mut out = format!(
            "/* The `{name}` group the gates the decode read select (docs/adr/0065\n     \
             * §6 R4). */\n    \
             memset(&{held}, 0, sizeof {held});\n"
        );
        let last = self.arms.len() - 1;
        for (i, arm) in self.group.arms.iter().enumerate() {
            let mut set = String::new();
            if self.group.absent.is_some() {
                set.push_str(&format!("        {held}.present = true;\n"));
            }
            if self.group.is_sum() {
                set.push_str(&format!("        {base}.kind = {};\n", self.arms[i].kind));
            }
            for id in &arm.fields {
                set.push_str(&format!(
                    "        {} = {};\n",
                    self.member_of(i, id, &held),
                    derived_local(id)
                ));
            }
            let keyword = if i == 0 { "    if" } else { " else if" };
            if i == last && self.group.absent.is_none() {
                if i == 0 {
                    out.push_str(&format!("    {{\n{set}    }}"));
                } else {
                    out.push_str(&format!(" else {{\n{set}    }}"));
                }
            } else {
                out.push_str(&format!(
                    "{keyword} ({}) {{\n{set}    }}",
                    self.decoded_test(plan, &arm.values)
                ));
            }
        }
        out.push('\n');
        out
    }

    /// Whether the gates of the group, as the decode read them, hold `values`.
    fn decoded_test(&self, plan: &EncodePlan<'_>, values: &[bool]) -> String {
        self.group
            .gates
            .iter()
            .zip(values)
            .map(|(gate, value)| {
                let compare = if *value { "!=" } else { "==" };
                match gate {
                    GateVar::Flag { carrier, flag } => {
                        let bit = plan
                            .model
                            .fields
                            .iter()
                            .find(|f| &f.id == carrier)
                            .and_then(|f| f.flags.iter().find(|d| &d.name == flag))
                            .map(|d| d.bit)
                            .expect("the parser found the flag a gate reads");
                        format!(
                            "((((uint64_t)out->{} >> {bit}) & 1u) {compare} 0)",
                            filters::to_snake_case(carrier.clone())
                        )
                    }
                    GateVar::Input(name) => format!("({} {compare} 0)", input_name(name)),
                    GateVar::Member { .. } => {
                        unreachable!("`unlowered` prints no group of a chain's memberships")
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" && ")
    }

    /// The arm of the group a byte case's source states, by its place among the
    /// group's arms: the statements that set it on `value` at the member path
    /// `at`, each member `(id, literal)` as its literal, and the checks that
    /// hold `decoded` to it, labelled `label`.
    pub(super) fn stated(
        &self,
        at: &str,
        arm: Option<usize>,
        members: &[(String, String)],
        label: &str,
        (value, decoded): (&str, &str),
    ) -> (Vec<String>, Vec<(String, String)>) {
        let held = format!("{at}{}", self.name);
        let mut statements = Vec::new();
        let mut checks = Vec::new();
        let Some(i) = arm else {
            checks.push((label.to_string(), format!("!{decoded}.{held}.present")));
            return (statements, checks);
        };
        let base = self.base(&held);
        if self.group.absent.is_some() {
            statements.push(format!("{value}.{held}.present = true;"));
            checks.push((label.to_string(), format!("{decoded}.{held}.present")));
        }
        if self.group.is_sum() {
            statements.push(format!("{value}.{base}.kind = {};", self.arms[i].kind));
            checks.push((
                format!("{label} arm"),
                format!("{decoded}.{base}.kind == {value}.{base}.kind"),
            ));
        }
        for (id, literal) in members {
            let member = self.member_of(i, id, &held);
            statements.push(format!("{value}.{member} = {literal};"));
            checks.push((
                format!("{label}.{id}"),
                format!("{decoded}.{member} == {value}.{member}"),
            ));
        }
        (statements, checks)
    }
}
