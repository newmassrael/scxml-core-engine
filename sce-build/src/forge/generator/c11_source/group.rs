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
//! value says a member is there that its gates leave out.
//!
//! An arm holds a number or an enum as its type, a byte string or a text in
//! the runtime's buffer of its bound (`sce_static_bytes_<N>_t`, the buffer a
//! value holds one a gate decides in), a body as its codec's struct, and a list
//! as the runtime's list of its entries (`{entry}_list_<N>_t`). A source
//! presents the group as the value holds it where every member is a number or
//! an enum; where some member is not, as `{codec}_{group}_ref_t`, the same
//! kind and arms, each member presented as an element of its type is (a view,
//! the pair of a body's source, a list over a walk), so a source presents a
//! body or a list where it is held, with no copy (docs/adr/0081 §2).
//!
//! The encode derives every gate of the group from the arm the source
//! presents, and binds each member to the optional value an element a gate
//! decides is read into, absent where the arm holds none, so each statement
//! that reads a member reads it as it reads a gated element. The decode reads
//! a number into a local of its own, as it reads a number the encode derives,
//! and sets it in the arm once every field is read; it reads a byte string, a
//! text, a body or a list in place, into the arm the gates it read select,
//! which it opens before the first of them, as the value holds no copy of one
//! anywhere else.

use super::super::rust_source::{group_accessor, group_arm_name};
use super::super::*;
use super::{
    body_prefix, derived_local, guarded, input_name, length_outside, BoundedList, BoundedView,
    LawText,
};
use crate::forge::codec_roles::{GateGroup, GateVar};
use crate::forge::encode_plan::{EncodePlan, Value};
use std::collections::BTreeMap;

/// What a member of a group holds, as the C11 printer holds and presents it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    /// A number or an enum: held and presented as its type, and read by the
    /// decode into a local of its own.
    Number,
    /// A byte string or a text: held in the runtime's buffer of its bound,
    /// presented as its view, and read by the decode in place.
    Bytes,
    /// A body: held as its codec's struct, presented as the pair of its
    /// source, and read by the decode in place.
    Body,
    /// A list: held as the runtime's list of its entries, presented as the
    /// list of its bound over a walk, and read by the decode in place.
    List,
}

impl Kind {
    /// What `value`, a member of a group, holds.
    fn of(value: &Value<'_>) -> Self {
        match value {
            Value::Scalar(_) => Kind::Number,
            Value::Bytes(_) | Value::Text(_) => Kind::Bytes,
            Value::Body { .. } => Kind::Body,
            Value::List { .. } => Kind::List,
        }
    }
}

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
    /// The tagged struct, or the one arm's struct, a source presents it as:
    /// [`Self::ty`] where every member is a number or an enum.
    presented_ty: String,
    /// The type a source presents it as: [`Self::presented_ty`], or its
    /// optional value where the gates allow no arm.
    pub(super) presented: String,
    /// What each member holds, by its id.
    kinds: BTreeMap<String, Kind>,
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
    /// Its struct as a source presents it.
    presented: String,
}

/// The groups of `plan`, a codec whose symbols are named after `snake`.
pub(super) fn groups<'p>(plan: &'p EncodePlan<'_>, snake: &str) -> Vec<Group<'p>> {
    plan.groups()
        .iter()
        .map(|g| Group::of(plan, snake, g))
        .collect()
}

/// The members of the groups of `plan` the decode reads in place, by id, each
/// with the pointer it reads it through: the place in the arm the decode
/// opened ([`Group::opened`]).
pub(super) fn in_place(plan: &EncodePlan<'_>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for group in plan.groups() {
        for id in group.fields() {
            if Kind::of(&plan.value(field_of(plan, id))) != Kind::Number {
                out.insert(id.to_string(), pointer(id));
            }
        }
    }
    out
}

/// The pointer the decode reads the member `id` in place through.
fn pointer(id: &str) -> String {
    format!("_g_{}", filters::to_snake_case(id.to_string()))
}

/// The field of `plan` a group holds as `id`.
fn field_of<'m>(plan: &EncodePlan<'m>, id: &str) -> &'m CodecField {
    plan.model
        .fields
        .iter()
        .find(|f| f.id == id)
        .expect("a member of a group is a field of the codec")
}

impl<'g> Group<'g> {
    /// `group`, a group of the codec `snake` whose plan is `plan`.
    fn of(plan: &EncodePlan<'_>, snake: &str, group: &'g GateGroup) -> Self {
        let name = group_accessor(group);
        let ty = format!("{snake}_{name}_t");
        let optional = group.absent.is_some();
        let kinds: BTreeMap<String, Kind> = group
            .fields()
            .into_iter()
            .map(|id| (id.to_string(), Kind::of(&plan.value(field_of(plan, id)))))
            .collect();
        let by_ref = kinds.values().any(|kind| *kind != Kind::Number);
        let presented_ty = if by_ref {
            format!("{snake}_{name}_ref_t")
        } else {
            ty.clone()
        };
        let (held, presented) = if optional {
            let presented = if by_ref {
                format!("{snake}_{name}_ref_optional_t")
            } else {
                format!("{snake}_{name}_optional_t")
            };
            (format!("{snake}_{name}_optional_t"), presented)
        } else {
            (ty.clone(), presented_ty.clone())
        };
        let arms = group
            .arms
            .iter()
            .map(|arm| {
                let member = filters::to_snake_case(group_arm_name(group, arm));
                let (arm_ty, arm_presented) = if group.is_sum() {
                    let arm_ty = format!("{snake}_{name}_{member}_t");
                    let arm_presented = if by_ref {
                        format!("{snake}_{name}_{member}_ref_t")
                    } else {
                        arm_ty.clone()
                    };
                    (arm_ty, arm_presented)
                } else {
                    (ty.clone(), presented_ty.clone())
                };
                ArmName {
                    kind: format!("{snake}_{name}_{member}").to_uppercase(),
                    ty: arm_ty,
                    presented: arm_presented,
                    member,
                }
            })
            .collect();
        Group {
            group,
            name,
            ty,
            held,
            presented_ty,
            presented,
            kinds,
            arms,
        }
    }

    /// Whether a source presents the group other than as the value holds
    /// it: some member is not a number or an enum.
    fn by_ref(&self) -> bool {
        self.presented_ty != self.ty
    }

    /// What the member `id` holds.
    fn kind(&self, id: &str) -> Kind {
        *self.kinds.get(id).expect("a member of a group has a kind")
    }

    /// The group's struct inside `held`, an expression of [`Self::held`] or
    /// of [`Self::presented`]: the value of the optional one, where the gates
    /// allow no arm.
    fn base(&self, held: &str) -> String {
        if self.group.absent.is_some() {
            format!("{held}.value")
        } else {
            held.to_string()
        }
    }

    /// Whether `held`, an expression of [`Self::held`] or of
    /// [`Self::presented`], holds arm `i`, or `None` where it holds that arm
    /// in every value.
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
    /// [`Self::held`] or of [`Self::presented`] that holds that arm: both
    /// lay their arms out alike.
    fn member_of(&self, i: usize, id: &str, held: &str) -> String {
        let base = self.base(held);
        let id = filters::to_snake_case(id.to_string());
        if self.group.is_sum() {
            format!("{base}.arm.{}.{id}", self.arms[i].member)
        } else {
            format!("{base}.{id}")
        }
    }

    /// The type the value holds the member `id` in, in the arm that holds
    /// it.
    fn held_type(&self, plan: &EncodePlan<'_>, l: &LangCtx, id: &str) -> String {
        let field = field_of(plan, id);
        let fields = &plan.model.fields;
        match plan.value(field) {
            Value::Scalar(ty) => l.type_name(ty).into_owned(),
            Value::Bytes(_) | Value::Text(_) => {
                crate::forge::static_lowering::c_bytes_storage_type(c11_held_bytes(field, fields))
            }
            Value::Body { codec } => format!("{}_t", body_prefix(codec)),
            Value::List { codec, .. } => crate::forge::static_lowering::c11_list_type(
                &format!("{}_t", body_prefix(codec)),
                c11_held_entries(field, fields),
            ),
        }
    }

    /// The type a source presents the member `id` as, in the arm that holds
    /// it: an element of its type is presented so.
    fn presented_type(&self, plan: &EncodePlan<'_>, l: &LangCtx, snake: &str, id: &str) -> String {
        let field = field_of(plan, id);
        let value = plan.value(field);
        match &value {
            Value::Scalar(ty) => l.type_name(ty).into_owned(),
            Value::Bytes(_) | Value::Text(_) => BoundedView::of(l, snake, field, &value).ty(),
            Value::Body { codec } => format!("{}_source_ref_t", body_prefix(codec)),
            Value::List { .. } => BoundedList::of(l, snake, field, &value).ty(),
        }
    }

    /// The optional value of [`Self::presented_type`], which the encode binds
    /// the member `id` to: `{prefix}_some(..)` and `{prefix}_none()` make one,
    /// `{prefix}` the type less its `_t`.
    pub(super) fn member_optional(
        &self,
        plan: &EncodePlan<'_>,
        l: &LangCtx,
        snake: &str,
        id: &str,
    ) -> String {
        let field = field_of(plan, id);
        let value = plan.value(field);
        match &value {
            Value::Scalar(ty) => l.c11_codec_optional(ty, None).type_name,
            Value::Bytes(_) | Value::Text(_) => BoundedView::of(l, snake, field, &value).optional(),
            Value::Body { codec } => format!("{}_source_ref_optional_t", body_prefix(codec)),
            Value::List { .. } => BoundedList::of(l, snake, field, &value).optional(),
        }
    }

    /// The typedefs of the group, before the value's struct: the list type
    /// of each list it holds, the kind of its arms, a struct for each arm,
    /// the tagged struct, and the optional value of it where the gates allow
    /// no arm. Each member is held in the arm that holds it, always there
    /// in that arm.
    pub(super) fn declare(&self, plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
        let (name, ty) = (&self.name, &self.ty);
        let mut out = String::new();
        // The runtime's list of each list's entries, declared once whoever
        // holds one of its shape.
        for id in self.group.fields() {
            let field = field_of(plan, id);
            if let Value::List { codec, .. } = plan.value(field) {
                let entry = format!("{}_t", body_prefix(codec));
                let capacity = c11_held_entries(field, &plan.model.fields);
                out.push_str(&crate::forge::static_lowering::c11_list_def(
                    &entry,
                    capacity,
                    &format!("A list of at most {capacity} entries of {entry}."),
                ));
                out.push_str("\n\n");
            }
        }
        let fields = |i: usize| -> String { self.arm_fields(i, |id| self.held_type(plan, l, id)) };
        self.declare_arms(&mut out, snake, ty, |arm| &arm.ty, fields);
        if self.group.absent.is_some() {
            out.push_str(&format!(
                "/* The `{name}` group of {snake}_t, absent where the gates allow no arm. */\n\
                 SCE_OPTIONAL_DECLARE_NAMED({snake}_{name}_optional, {ty})\n\n"
            ));
        }
        out
    }

    /// The typedefs a source presents the group by, after the value's
    /// struct: the view of each byte string and text and the list of each
    /// list it holds, with their optional values, which the encode binds a
    /// member to; and, where a source presents the group other than as the
    /// value holds it, a struct for each arm, the tagged struct and the
    /// optional value of it where the gates allow no arm.
    pub(super) fn declare_presented(
        &self,
        plan: &EncodePlan<'_>,
        l: &LangCtx,
        snake: &str,
    ) -> String {
        let value_ty = format!("{snake}_t");
        let mut out = String::new();
        for id in self.group.fields() {
            let field = field_of(plan, id);
            let value = plan.value(field);
            match &value {
                Value::Bytes(_) | Value::Text(_) => {
                    let view = BoundedView::of(l, snake, field, &value);
                    out.push_str(&view.declare(id, &value_ty));
                    out.push_str(&view.declare_optional());
                }
                // A list a group holds is held to no law
                // (`c11_source::positional_forms`).
                Value::List { .. } => {
                    let list = BoundedList::of(l, snake, field, &value);
                    out.push_str(&list.declare(id, &value_ty, &LawText::default()));
                    out.push_str(&list.declare_optional());
                }
                Value::Scalar(_) | Value::Body { .. } => {}
            }
        }
        if !self.by_ref() {
            return out;
        }
        let fields = |i: usize| -> String {
            self.arm_fields(i, |id| self.presented_type(plan, l, snake, id))
        };
        let ty = self.presented_ty.clone();
        self.declare_arms(&mut out, snake, &ty, |arm| &arm.presented, fields);
        if self.group.absent.is_some() {
            let name = &self.name;
            out.push_str(&format!(
                "/* The `{name}` group a source of {snake}_t presents, absent where the\n \
                 * gates allow no arm. */\n\
                 SCE_OPTIONAL_DECLARE_NAMED({snake}_{name}_ref_optional, {ty})\n\n"
            ));
        }
        out
    }

    /// The members of arm `i`, each a line of its struct of the type `ty`
    /// answers for its id.
    fn arm_fields(&self, i: usize, ty: impl Fn(&str) -> String) -> String {
        let arm = &self.group.arms[i];
        if arm.fields.is_empty() {
            return "    /* A C struct declares at least one member; the arm holds\n     \
                    * none. */\n    char _reserved;\n"
                .to_string();
        }
        arm.fields
            .iter()
            .map(|id| format!("    {} {};\n", ty(id), filters::to_snake_case(id.clone())))
            .collect()
    }

    /// Into `out`, the kind of the arms where the group has several, the
    /// struct of each arm, named as `arm_ty` answers and holding the lines
    /// `fields` answers, and the tagged struct `ty`; or the one arm's struct
    /// `ty`. The kind is declared once, for the value's struct, and a source's
    /// struct reads it there.
    fn declare_arms(
        &self,
        out: &mut String,
        snake: &str,
        ty: &str,
        arm_ty: impl Fn(&ArmName) -> &String,
        fields: impl Fn(usize) -> String,
    ) {
        let name = &self.name;
        let presented = ty != self.ty;
        let whose = if presented {
            format!("a source of {snake}_t presents")
        } else {
            format!("of {snake}_t")
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
        if self.group.is_sum() {
            if !presented {
                let kinds: String = self
                    .arms
                    .iter()
                    .map(|arm| format!("    {},\n", arm.kind))
                    .collect();
                out.push_str(&format!(
                    "/* The arms of the `{name}` group of {snake}_t (docs/adr/0065 §6 R4). */\n\
                     typedef enum {{\n{kinds}}} {snake}_{name}_kind_t;\n\n"
                ));
            }
            let mut union = String::new();
            for (i, arm) in self.arms.iter().enumerate() {
                let arm_ty = arm_ty(arm);
                out.push_str(&format!(
                    "/* The `{name}` group {whose} where {}: the members present\n \
                     * then. */\n\
                     typedef struct {{\n{}}} {arm_ty};\n\n",
                    when(i),
                    fields(i),
                ));
                union.push_str(&format!("        {arm_ty} {};\n", arm.member));
            }
            out.push_str(&format!(
                "/* The `{name}` group {whose}: the arm the value holds, which `kind`\n \
                 * names, each the members present while the gates hold its values\n \
                 * (docs/adr/0065 §6 R4). */\n\
                 typedef struct {{\n    {snake}_{name}_kind_t kind;\n    union {{\n{union}    \
                 }} arm;\n}} {ty};\n\n"
            ));
        } else {
            out.push_str(&format!(
                "/* The `{name}` group {whose} where {}: the members present then\n \
                 * (docs/adr/0065 §6 R4). */\n\
                 typedef struct {{\n{}}} {ty};\n\n",
                when(0),
                fields(0)
            ));
        }
    }

    /// The function of the value's table that presents the group, after the
    /// value's struct: the group as the struct holds it, or, where a source
    /// presents it otherwise, each member of the arm the struct holds
    /// presented where it is held, a view of a buffer, the pair of a body's
    /// source over the body, and a list over a walk of the list the arm holds,
    /// whose begin and next come first.
    pub(super) fn reader(&self, plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
        let name = &self.name;
        let value = format!("{snake}_t");
        if !self.by_ref() {
            return format!(
                "/* The `{name}` group of the {value} at `ctx`. */\n\
                 static inline {held} {snake}_value_{name}(const void *ctx) {{\n    \
                 return ((const {value} *)ctx)->{name};\n\
                 }}\n\n",
                held = self.held
            );
        }
        let held = format!("self->{name}");
        let mut walks = String::new();
        let mut statics = String::new();
        let mut arms = String::new();
        for (i, arm) in self.group.arms.iter().enumerate() {
            let mut set = String::new();
            for id in &arm.fields {
                let from = self.member_of(i, id, &held);
                let to = self.member_of(i, id, "ref");
                let field = field_of(plan, id);
                let member = plan.value(field);
                match &member {
                    Value::Scalar(_) => set.push_str(&format!("    {to} = {from};\n")),
                    Value::Bytes(_) | Value::Text(_) => {
                        let view = BoundedView::of(l, snake, field, &member);
                        let data = if view.text {
                            format!("(const char *){from}.data")
                        } else {
                            format!("{from}.data")
                        };
                        set.push_str(&format!(
                            "    {to}.data = {data};\n    {to}.len = {from}.len;\n"
                        ));
                    }
                    Value::Body { codec } => set.push_str(&format!(
                        "    {to}.table = {}_value_source();\n    {to}.ctx = &{from};\n",
                        body_prefix(codec)
                    )),
                    Value::List { codec, .. } => {
                        let entry = body_prefix(codec);
                        let walk = self.walk(snake, i, id);
                        let label = filters::to_snake_case(id.clone());
                        walks.push_str(&format!(
                            "/* The first entry of the `{label}` of the `{name}` group of the\n \
                             * {value} at `ctx`, or NULL where it holds none. */\n\
                             static inline const void *{walk}_begin(const void *ctx) {{\n    \
                             const {value} *self = (const {value} *)ctx;\n    \
                             return {from}.len > 0u ? (const void *)&{from}.data[0] : NULL;\n\
                             }}\n\n\
                             /* The entry after `entry` of the `{label}` of the `{name}` group of\n \
                             * the {value} at `ctx`, or NULL where `entry` is its last. */\n\
                             static inline const void *{walk}_next(const void *ctx, \
                             const void *entry) {{\n    \
                             const {value} *self = (const {value} *)ctx;\n    \
                             const {entry}_t *at = (const {entry}_t *)entry + 1;\n    \
                             return at < {from}.data + {from}.len ? (const void *)at : NULL;\n\
                             }}\n\n"
                        ));
                        statics.push_str(&format!(
                            "    static const sce_forge_walk_t {walk}_walk = {{\n        \
                             {walk}_begin,\n        {walk}_next,\n    }};\n"
                        ));
                        set.push_str(&format!(
                            "    {to}.walk = &{walk}_walk;\n    {to}.ctx = ctx;\n    \
                             {to}.entry = {entry}_value_source();\n    {to}.count = {from}.len;\n"
                        ));
                    }
                }
            }
            if set.is_empty() {
                continue;
            }
            arms.push_str(&match self.arm_test(i, &held) {
                Some(test) => guarded(&test, &set),
                None => set,
            });
        }
        let mut open = String::new();
        if self.group.absent.is_some() {
            open.push_str(&format!(
                "    if (!{held}.present) {{\n        return ref;\n    }}\n    ref.present = true;\n"
            ));
        }
        if self.group.is_sum() {
            open.push_str(&format!(
                "    {}.kind = {}.kind;\n",
                self.base("ref"),
                self.base(&held)
            ));
        }
        format!(
            "{walks}/* The `{name}` group of the {value} at `ctx`, each member presented\n \
             * where the value holds it. */\n\
             static inline {presented} {snake}_value_{name}(const void *ctx) {{\n\
             {statics}    const {value} *self = (const {value} *)ctx;\n    \
             {presented} ref;\n    \
             memset(&ref, 0, sizeof ref);\n\
             {open}{arms}    return ref;\n\
             }}\n\n",
            presented = self.presented
        )
    }

    /// The stem of the walk of the list `id` of arm `i`, over the value.
    fn walk(&self, snake: &str, i: usize, id: &str) -> String {
        let id = filters::to_snake_case(id.to_string());
        if self.group.is_sum() {
            format!("{snake}_value_{}_{}_{id}", self.name, self.arms[i].member)
        } else {
            format!("{snake}_value_{}_{id}", self.name)
        }
    }

    /// What the encode binds the member `id` to, off `held`, the group as a
    /// source presents it: the optional value of what it presents
    /// ([`Self::member_optional`]), present where the arm `held` holds holds
    /// `id` and absent where it holds none.
    pub(super) fn member_read(
        &self,
        plan: &EncodePlan<'_>,
        l: &LangCtx,
        snake: &str,
        id: &str,
        held: &str,
    ) -> String {
        let optional = self.member_optional(plan, l, snake, id);
        let prefix = optional.trim_end_matches("_t");
        let mut read = format!("{prefix}_none()");
        for (i, arm) in self.group.arms.iter().enumerate().rev() {
            if !arm.fields.iter().any(|f| f == id) {
                continue;
            }
            let some = format!("{prefix}_some({})", self.member_of(i, id, held));
            read = match self.arm_test(i, held) {
                Some(test) => format!("({test} ? {some} : {read})"),
                None => some,
            };
        }
        read
    }

    /// Whether the gate at `gate` among the group's holds where a source
    /// presents `held`, the group as it presents it: the value holds an arm
    /// that sets it, or no arm where the gates the value then has set it. A C
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

    /// The statements of the value's check that hold each member of the arm
    /// the value holds to what its type holds: a byte string or a text within
    /// its view's bound and the buffer's, a text UTF-8, a list within its
    /// bound and the list's and each entry a value of its codec, and a body a
    /// value of its codec, each by the check the element of its type has
    /// (docs/adr/0081 §2). Empty where every member is a number or an enum.
    pub(super) fn check(&self, plan: &EncodePlan<'_>, l: &LangCtx, snake: &str) -> String {
        let held = format!("self->{}", self.name);
        let mut out = String::new();
        for (i, arm) in self.group.arms.iter().enumerate() {
            let mut checks = String::new();
            for id in &arm.fields {
                let at = self.member_of(i, id, &held);
                let field = field_of(plan, id);
                let member = plan.value(field);
                match &member {
                    Value::Scalar(_) => {}
                    Value::Bytes(_) | Value::Text(_) => {
                        let view = BoundedView::of(l, snake, field, &member);
                        let capacity = u64::from(c11_held_bytes(field, &plan.model.fields));
                        let most = view.bound.most.map_or(capacity, |most| most.min(capacity));
                        if let Some(outside) =
                            length_outside(&format!("{at}.len"), view.bound.least, Some(most))
                        {
                            checks.push_str(&format!(
                                "    if ({outside}) {{\n        \
                                 return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }}\n"
                            ));
                        }
                        if view.text {
                            checks.push_str(&format!(
                                "    if (!sce_forge_is_valid_utf8({at}.data, {at}.len)) {{\n        \
                                 return SCE_FORGE_CODEC_INVALID_UTF8;\n    }}\n"
                            ));
                        }
                    }
                    Value::Body { codec } => checks.push_str(&format!(
                        "    SCE_FORGE_TRY_WRITE({}_check_value(&{at}));\n",
                        body_prefix(codec)
                    )),
                    Value::List { codec, .. } => {
                        let list = BoundedList::of(l, snake, field, &member);
                        let capacity = u64::from(c11_held_entries(field, &plan.model.fields));
                        let most = list.bound.most.map_or(capacity, |most| most.min(capacity));
                        checks.push_str(&format!(
                            "    if (!sce_forge_length_within_bound({at}.len, {most})) {{\n        \
                             return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;\n    }}\n"
                        ));
                        if list.bound.least > 0 {
                            checks.push_str(&format!(
                                "    if ({at}.len < {}u) {{\n        \
                                 return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;\n    }}\n",
                                list.bound.least
                            ));
                        }
                        checks.push_str(&format!(
                            "    for (size_t _i = 0; _i < {at}.len; ++_i) {{\n        \
                             SCE_FORGE_TRY_WRITE({}_check_value(&{at}.data[_i]));\n    }}\n",
                            body_prefix(codec)
                        ));
                    }
                }
            }
            if checks.is_empty() {
                continue;
            }
            out.push_str(&match self.arm_test(i, &held) {
                Some(test) => guarded(&test, &checks),
                None => checks,
            });
        }
        out
    }

    /// Where the decode reads some member of the group in place, the field
    /// before whose decode it opens the group and the statements that open
    /// it: the group zeroed, the arm the gates it read select set, and a
    /// pointer to each member read in place set to its place in that arm, or
    /// left null where the arm holds none, whose own gate then leaves it
    /// unread. Every gate of the group is a flag of a carrier the decode has
    /// read, or an input it is handed, before the first member a gate decides
    /// (docs/adr/0065 §6 R4b, R4c). Each statement after the first is
    /// indented as the decode's own; the template indents the first.
    pub(super) fn opened(&self, plan: &EncodePlan<'_>, l: &LangCtx) -> Option<(String, String)> {
        let first = plan.model.fields.iter().find(|f| {
            self.kinds
                .get(&f.id)
                .is_some_and(|kind| *kind != Kind::Number)
        })?;
        let name = &self.name;
        let held = format!("out->{name}");
        let mut out = format!(
            "/* The `{name}` group the gates the decode read select (docs/adr/0065\n     \
             * §6 R4), opened before the members it reads in place. */\n    \
             memset(&{held}, 0, sizeof {held});\n"
        );
        for id in self.group.fields() {
            if self.kind(id) != Kind::Number {
                out.push_str(&format!(
                    "    {} *{} = NULL;\n",
                    self.held_type(plan, l, id),
                    pointer(id)
                ));
            }
        }
        out.push_str(&self.select(plan, |i| {
            let mut set = self.arm_set(i, &held);
            for id in &self.group.arms[i].fields {
                if self.kind(id) != Kind::Number {
                    set.push_str(&format!(
                        "        {} = &{};\n",
                        pointer(id),
                        self.member_of(i, id, &held)
                    ));
                }
            }
            set
        }));
        Some((first.id.clone(), out.trim_end().to_string()))
    }

    /// The statements that make the value's group of what the decode of the
    /// codec `plan` read, once every field is read. Where the decode read no
    /// member in place: the arm the gates it read select, each flag tested on
    /// the carrier the decode read it into and each input as the decode is
    /// handed it, no arm where they select none, and each member set from the
    /// local the decode read it into ([`derived_local`]). Where it read some
    /// in place, it opened the arm before them ([`Self::opened`]), and each
    /// number is set in the arm it opened; empty where the group holds no
    /// number.
    pub(super) fn decoded(&self, plan: &EncodePlan<'_>) -> String {
        let name = &self.name;
        let held = format!("out->{name}");
        let numbers = |i: usize, indent: &str| -> String {
            self.group.arms[i]
                .fields
                .iter()
                .filter(|id| self.kind(id) == Kind::Number)
                .map(|id| {
                    format!(
                        "{indent}{} = {};\n",
                        self.member_of(i, id, &held),
                        derived_local(id)
                    )
                })
                .collect()
        };
        if self.kinds.values().any(|kind| *kind != Kind::Number) {
            let mut sets = String::new();
            for i in 0..self.group.arms.len() {
                let set = numbers(i, "        ");
                if set.is_empty() {
                    continue;
                }
                sets.push_str(&match self.arm_test(i, &held) {
                    Some(test) => format!("    if ({test}) {{\n{set}    }}\n"),
                    None => numbers(i, "    "),
                });
            }
            if sets.is_empty() {
                return String::new();
            }
            return format!(
                "/* The numbers of the `{name}` group, read into locals, set in the arm\n     \
                 * the decode opened (docs/adr/0065 §6 R4). */\n{sets}"
            );
        }
        let mut out = format!(
            "/* The `{name}` group the gates the decode read select (docs/adr/0065\n     \
             * §6 R4). */\n    \
             memset(&{held}, 0, sizeof {held});\n"
        );
        out.push_str(&self.select(plan, |i| {
            let mut set = self.arm_set(i, &held);
            set.push_str(&numbers(i, "        "));
            set
        }));
        out
    }

    /// The statements that say the decode's value of the group, `held`, holds
    /// arm `i`: present where the gates allow no arm, and its kind where the
    /// group has several.
    fn arm_set(&self, i: usize, held: &str) -> String {
        let mut set = String::new();
        if self.group.absent.is_some() {
            set.push_str(&format!("        {held}.present = true;\n"));
        }
        if self.group.is_sum() {
            set.push_str(&format!(
                "        {}.kind = {};\n",
                self.base(held),
                self.arms[i].kind
            ));
        }
        set
    }

    /// The arm the gates the decode of `plan` read select, each arm's
    /// statements those `set` answers for its place: an `if` for each arm the
    /// gates may not select, an `else` for the last where they select one
    /// always, and nothing where they select none.
    fn select(&self, plan: &EncodePlan<'_>, set: impl Fn(usize) -> String) -> String {
        let mut out = String::new();
        let last = self.arms.len() - 1;
        for (i, arm) in self.group.arms.iter().enumerate() {
            let set = set(i);
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

    /// The arm of the group a byte case's source states, by its place among
    /// the group's arms, or `None` for no arm: the statements that set it on
    /// `value` at the member path `at`, its presence and its kind, and the
    /// checks that hold `decoded` to it, labelled `label`. Each member of the
    /// arm is set at [`Self::member_path`].
    pub(super) fn stated(
        &self,
        at: &str,
        arm: Option<usize>,
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
        (statements, checks)
    }

    /// The member path of the member `id` of arm `i` of the group held at
    /// the member path `at` of a value.
    pub(super) fn member_path(&self, i: usize, id: &str, at: &str) -> String {
        self.member_of(i, id, &format!("{at}{}", self.name))
    }
}
