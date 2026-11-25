// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust spelling of the walks over a composed value ([`super`]), for a
//! machine (docs/adr/0086, section 7.1, the Rust machine row), and the Rust
//! spelling of the reads of a union that a schema unit and a machine share.
//!
//! | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record |
//! |---|---|---|---|
//! | `BoundedList<T, N>` | `SceString<N>` / `SceBytes<N>` | `Option<T>` | the machine's record |
//!
//! A list, a text and a byte string carry their bound in their type (section
//! 7.4), so a value is copied into a place of a larger bound by widening it at
//! each depth where the two bounds differ, the fit proven before ([`copier`],
//! section 5.1) and asserted again by the widening's own const parameters.

use super::*;

/// The identifier of the case `variant` of a union's type: the variant as an
/// enum's Rust declaration spells it ([`crate::forge::enum_naming::variant_ident`]).
pub(crate) fn rust_case_ident(variant: &str) -> String {
    crate::forge::enum_naming::variant_ident(crate::generator::Language::Rust, "", variant)
}

/// The read of the record of the case `variant` where a test proved it, on a
/// machine's own union, which declares it (`RustComposed`'s `union_def`).
pub(crate) fn rust_case_read(variant: &str) -> String {
    format!(
        "proven_{}",
        crate::filters::to_snake_case(variant.to_string())
    )
}

/// The view of the record of the case `variant` on a union's unit
/// (docs/adr/0092, section 7, the Rust row): `as_<case>(&self) -> Option<&R>`,
/// `None` in another case, so the unit holds no panic.
pub(crate) fn rust_case_view(variant: &str) -> String {
    format!("as_{}", crate::filters::to_snake_case(variant.to_string()))
}

/// The forge runtime's read of a case a test of the union's tag proved, over the
/// case's view ([`rust_case_view`]): the one arm of the other cases, which no
/// run reaches, is the runtime's, not the unit's.
const RUST_UNION_CASE_PROVEN: &str = "sce_forge_runtime::union_case::proven";

/// The read of a field several cases share ([`SharedFieldDef`]) on a machine's
/// union.
pub(crate) fn rust_shared_read(field: &str) -> String {
    format!(
        "proven_field_{}",
        crate::filters::to_snake_case(field.to_string())
    )
}

/// The methods every value of a generated Rust union answers through its
/// derives (`Clone`, `PartialEq`, `Debug`): an inherent method of one of these
/// names would hide the trait's at every call written with a dot, so the tag's
/// accessor is never one of them.
const RUST_UNION_DERIVED_METHODS: &[&str] = &["clone", "clone_from", "eq", "ne", "fmt"];

/// The Rust method the tag's key `tag` is read by on a generated union, the
/// unit's and the machine's alike: the key in snake case, as a payload's field
/// is named, escaped with `r#` where Rust reserves it. `None` for a key that
/// is no Rust identifier, one `r#` cannot escape (`self`, `Self`, `super`,
/// `crate`), and one that would hide a derived method
/// ([`RUST_UNION_DERIVED_METHODS`]).
pub(crate) fn rust_union_tag_method(tag: &str) -> Option<String> {
    let snake = crate::filters::to_snake_case(tag.to_string());
    let mut chars = snake.chars();
    let identifier = chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && snake != "_";
    if !identifier
        || ["self", "Self", "super", "crate"].contains(&snake.as_str())
        || RUST_UNION_DERIVED_METHODS.contains(&snake.as_str())
    {
        return None;
    }
    Some(if crate::filters::RUST_KEYWORDS.contains(&snake.as_str()) {
        format!("r#{snake}")
    } else {
        snake
    })
}

/// The Rust spelling of what follows a union's value to read `member` where
/// it may be in the cases `left` ([`ComposedSpelling::union_member`]), as
/// [`union_member_of`] reads it: the tag's method; the field of the one case
/// left, read through that case's proven read, a place as a record's field
/// is; or the accessor of a field several cases share. `Err` as
/// [`union_member_of`]'s, and for a tag's key no Rust method is named by.
pub(crate) fn rust_union_member(
    union: &ScopedUnion,
    left: &[&ScopedCase],
    member: &str,
) -> Result<String, String> {
    match union_member_of(union, left, member)? {
        UnionMember::Tag => rust_union_tag_method(member)
            .map(|method| format!("{method}()"))
            .ok_or_else(|| {
                format!("the tag's key `{member}` names no Rust method its value can be read by")
            }),
        UnionMember::Case(case) => Ok(format!("{}().{member}", rust_case_read(&case.variant.name))),
        UnionMember::Shared => Ok(format!("{}()", rust_shared_read(member))),
    }
}

/// The Rust method a read of a machine's union (`proven_<case>`,
/// `proven_field_<f>`) is written through: the read's name with `_mut`, a
/// mutable reference to the same place.
pub(crate) fn rust_union_place_of(read: &str) -> String {
    format!("{read}_mut")
}

/// The Rust place of the field `member` of the union value `holder` where it
/// may be in the cases `left` ([`ComposedSpelling::union_member_place`]): the
/// field of the one case left, through that case's mutable proven read, or
/// the place a field several cases share is written through
/// ([`shared_plain_field`]). `Err` for the tag, which is never written alone
/// (docs/adr/0092, section 6.1 item 6), and for a field the cases left share
/// whose value is not copied as it is read, which no accessor answers yet.
pub(crate) fn rust_union_member_place(
    holder: &str,
    union: &ScopedUnion,
    left: &[&ScopedCase],
    member: &str,
) -> Result<String, String> {
    if member == union.tag {
        return Err(format!(
            "the tag `{member}` of `union:{}` is never written alone: the case changes by a \
             whole assignment (docs/adr/0092, section 6.1 item 6)",
            union.name
        ));
    }
    // The read stands exactly where the write does: one case left that
    // declares the field, or several that share it.
    rust_union_member(union, left, member)?;
    Ok(match left {
        [case] => format!(
            "{holder}.{}().{member}",
            rust_union_place_of(&rust_case_read(&case.variant.name))
        ),
        _ => format!(
            "*{holder}.{}()",
            rust_union_place_of(&rust_shared_read(member))
        ),
    })
}

/// The Rust read of `member` of the union value `holder`, a parameter or a
/// local of an algorithm spelled as Rust names it, whose type is the union's
/// own unit, `ty` (docs/adr/0092, section 7, the Rust row, and Landing order
/// step 5), where the value may be in the cases `left`, as
/// [`unit_union_member_of`] classifies it: the tag's method; the field of the
/// one case left, through the forge runtime's `union_case::proven` of that
/// case's view; or the field the cases left each declare, read by a `match`
/// over them, each arm the field, where every case is left, and otherwise the
/// runtime's `union_case::proven` of a `match` whose other arm, which the
/// narrowing proved is never taken, is `None`. The unit's views are total, so
/// the one arm no run reaches is the runtime's. A case's record is the payload
/// type its event schema's unit declares, whose fields are named in snake case
/// ([`crate::forge::generator::event_schema_field_ident`]). `Err` as
/// [`unit_union_member_of`]'s, and for a tag's key no Rust method is named by.
pub(crate) fn rust_unit_union_read(
    holder: &str,
    ty: &str,
    union: &ScopedUnion,
    left: &[&ScopedCase],
    member: &str,
) -> Result<String, String> {
    use crate::forge::expr::rust_tree::{RustArm, RustExpr, RustUnOp};
    let field =
        crate::forge::generator::event_schema_field_ident(member, crate::generator::Language::Rust);
    let held = || RustExpr::atom(holder);
    let read = match unit_union_member_of(union, left, member)? {
        UnionMember::Tag => rust_union_tag_method(member)
            .map(|method| held().method(method, Vec::new()))
            .ok_or_else(|| {
                format!("the tag's key `{member}` names no Rust method its value can be read by")
            })?,
        UnionMember::Case(case) => RustExpr::path_call(
            RUST_UNION_CASE_PROVEN,
            vec![held().method(rust_case_view(&case.variant.name), Vec::new())],
        )
        .field(field),
        UnionMember::Shared => {
            // Every case is an arm where every case is left, and the `match`
            // is the field. Otherwise the narrowing proved the value is in none
            // of the others, whose arm is `None`, and the runtime reads the
            // field out of the `Some` of the case the value is in.
            let others = unit_union_leaves_others(union, left);
            let mut arms: Vec<RustArm> = left
                .iter()
                .map(|case| {
                    let read = RustExpr::atom("record").field(field.as_str());
                    RustArm {
                        pattern: format!("{ty}::{}(record)", rust_case_ident(&case.variant.name)),
                        body: if others {
                            RustExpr::path_call("Some", vec![read])
                        } else {
                            read
                        },
                    }
                })
                .collect();
            let scrutinee = held().prefixed(RustUnOp::Ref);
            if others {
                arms.push(RustArm {
                    pattern: "_".to_string(),
                    body: RustExpr::atom("None"),
                });
                RustExpr::path_call(
                    RUST_UNION_CASE_PROVEN,
                    vec![RustExpr::match_of(scrutinee, arms, true)],
                )
            } else {
                RustExpr::match_of(scrutinee, arms, true)
            }
        }
    };
    // The read stands where the expression emitter splices it as it is
    // (`ExprKind::Raw`), so it is printed as one operand.
    Ok(read.as_operand())
}

/// The Rust spellings of the walks, for the machine `machine` (section 7.1,
/// the machine column): the runtime's `PayloadValue` reads each constructor,
/// and its writers write them into `carrier`.
pub struct RustComposed<'a> {
    pub machine: &'a str,
    pub carrier: RustCarrier,
}

/// What a Rust writer spells a composed value into: one walk of the value's
/// type, written as the carrier the place that takes it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustCarrier {
    /// The JSON text itself, by the runtime's `list_json`, `optional_json`,
    /// `record_json` and `union_json`: what the inject seam hands a lift
    /// (section 3.1).
    Json,
    /// The runtime's `ScriptValue`, a list as its `Array` and a record as its
    /// `Object`, every scalar as a `<send>`'s scalar param is
    /// (`StaticTarget::wire_value`): what a `<send>`'s params are held in until
    /// the runtime writes them as one JSON object, a record's keys ascending by
    /// their UTF-8 bytes at every depth (section 5.3; ARCHITECTURE.md, "JSON
    /// Object Key Order").
    ScriptValue,
}

/// The runtime module the Rust readers and writers are in.
const RUST_PAYLOAD: &str = "::sce_rust_runtime::event_payload";

/// The runtime's dynamic value, which a Rust `<send>` holds its params in.
const RUST_SCRIPT_VALUE: &str = "::sce_rust_runtime::ScriptValue";

/// `members`, each a key and the `ScriptValue` its value is, as one
/// `ScriptValue::Object`: the runtime orders its keys when it writes it.
fn rust_script_object(members: &[(String, String)]) -> String {
    let members: Vec<String> = members
        .iter()
        .map(|(id, value)| format!("(\"{id}\".to_string(), {value})"))
        .collect();
    format!(
        "{RUST_SCRIPT_VALUE}::Object(vec![{}].into_iter().collect())",
        members.join(", ")
    )
}

/// A union value held at `held`, of the machine's sum `ty`, as one
/// `ScriptValue::Object`: the tag `tag`, its value the case's name, beside the
/// fields of the case's record (docs/adr/0092, section 2.1), as the runtime's
/// `union_json` writes the same value.
fn rust_script_union(held: &Held, ty: &str, tag: &str, arms: &[UnionWriteArm]) -> String {
    let tag = crate::filters::escape_rust(tag.to_string());
    let arms: String = arms
        .iter()
        .map(|arm| {
            let named = (
                tag.clone(),
                format!(
                    "{RUST_SCRIPT_VALUE}::String(\"{}\".to_string())",
                    crate::filters::escape_rust(arm.variant.clone())
                ),
            );
            match &arm.fields {
                Some(fields) => {
                    let mut members = fields.clone();
                    members.push(named);
                    format!(
                        "{ty}::{}(h) => {}, ",
                        arm.case_ident,
                        rust_script_object(&members)
                    )
                }
                None => format!(
                    "{ty}::{} => {}, ",
                    arm.case_ident,
                    rust_script_object(&[named])
                ),
            }
        })
        .collect();
    format!("match {} {{ {arms}}}", held.borrowed())
}

/// The value `held` is, where it is a reference to the value: a copy of a
/// number or a truth value read through it.
fn rust_copied(held: &Held) -> String {
    if held.by_reference {
        format!("*{}", held.text)
    } else {
        held.text.clone()
    }
}

/// The crate of the bounded storage a Rust machine holds a list, a text and a
/// byte string in.
///
/// Named through the runtime every machine already depends on
/// (`sce_rust_runtime::portable`, the crate re-exported whole), so a crate that
/// holds a machine needs no dependency of its own on `sce-portable-bytes`. A
/// schema unit names the same types as `::sce_portable_bytes`, a crate a forge
/// unit depends on directly; the two paths are one type.
pub(crate) const RUST_PORTABLE: &str = "::sce_rust_runtime::portable";

/// The type a Rust machine holds a list of at most `max` values held in `elem`
/// in, every place alike (docs/adr/0086, section 7.4): a variable, a record's
/// field and a payload's, so two places of one list type are one Rust type.
pub(crate) fn rust_bounded_list(elem: &str, max: u32) -> String {
    format!("{RUST_PORTABLE}::BoundedList<{elem}, {max}>")
}

/// The type a Rust machine holds a text of at most `max` bytes in, every place
/// alike (docs/adr/0086, section 7.4).
pub(crate) fn rust_text_type(max: u32) -> String {
    format!("{RUST_PORTABLE}::SceString<{max}>")
}

/// The type a Rust machine holds a byte string of at most `max` bytes in,
/// every place alike (docs/adr/0086, section 7.4).
pub(crate) fn rust_bytes_type(max: u32) -> String {
    format!("{RUST_PORTABLE}::SceBytes<{max}>")
}

/// The empty list a Rust machine starts a list variable at, its element and
/// bound taken from the place ([`rust_bounded_list`]).
pub(crate) const RUST_EMPTY_LIST: &str = "::sce_rust_runtime::portable::BoundedList::new()";

/// The runtime module a Rust machine's saved state is written and read by.
const RUST_SAVED: &str = "::sce_rust_runtime::saved_state";

/// The Rust read of the saved value `value` (an expression of `&Value`), named
/// `what` in a refusal (an expression of `&str`), as a value of `ty`, where
/// `ty` states a lower count the type the machine holds it in does not hold: a
/// `BoundedList<T, N>` holds its bound and no lower count (docs/adr/0086,
/// section 15.6), so each list that states one is read by the runtime's
/// `bounded_within`, held to both its counts before an element is read, and
/// each value that holds one by an element or a present value, at any depth
/// down to a record or a union, whose own `SavedValue` reads its fields.
/// `None` where `ty` states none above such a record or union: the type's own
/// `SavedValue` reads it whole. `depth` names the parameters of each nested
/// closure, so none hides its holder's.
pub(crate) fn rust_saved_read(
    ty: &ValueType,
    value: &str,
    what: &str,
    depth: usize,
) -> Option<String> {
    match ty {
        ValueType::Optional { elem } => {
            let present = format!("sce_present{depth}");
            rust_saved_read(elem, &present, what, depth + 1)
                .map(|read| format!("{RUST_SAVED}::optional({value}, |{present}| {read})"))
        }
        ValueType::List { elem, max } => {
            let (item, item_what) = (format!("sce_item{depth}"), format!("sce_what{depth}"));
            let each = rust_saved_read(elem, &item, &item_what, depth + 1);
            if max.min == 0 && each.is_none() {
                return None;
            }
            // An element that states no lower count is read by its own
            // `SavedValue`, named by the function's path.
            let each = each.map_or_else(
                || format!("{RUST_SAVED}::SavedValue::from_saved"),
                |read| format!("|{item}, {item_what}| {read}"),
            );
            Some(format!(
                "{RUST_SAVED}::bounded_within::<_, {}>({value}, {what}, {}, {each})",
                max.max, max.min
            ))
        }
        // No machine holds a map yet (docs/adr/0108, D10): the type the
        // machine holds a value in refuses one by name before any read of it
        // is written ([`machine_type`]), and its saved form lands with it.
        ValueType::Map { .. } | ValueType::Set { .. } => None,
        _ => None,
    }
}

impl ComposedSpelling for RustComposed<'_> {
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::rust_type(ty).to_string()
    }
    // The portable bounded storage, `N` on the type (docs/adr/0086, section 7.1).
    fn text_type(&self, max: u32) -> String {
        rust_text_type(max)
    }
    fn bytes_type(&self, max: u32) -> String {
        rust_bytes_type(max)
    }
    fn optional_type(&self, elem: &str) -> String {
        format!("Option<{elem}>")
    }
    fn list_type(&self, elem: &str, max: u32) -> String {
        rust_bounded_list(elem, max)
    }
    // The name a statechart's record variable of the same schema is held in
    // (`StaticTarget::record_type`), so a record a payload holds is the type a
    // variable holds.
    fn record_type(&self, alias: &str) -> String {
        format!(
            "{}{}Record",
            self.machine,
            crate::filters::to_pascal_case(alias.to_string())
        )
    }
    // Owned and compared whole, as a record of entries that owns a buffer is;
    // no default, since an enum a field holds has none. Every bound is held by
    // a member's type (section 7.4), so there is nothing to check.
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        fields: &[(String, String)],
        _checks: &[String],
    ) -> String {
        let members: String = fields
            .iter()
            .map(|(id, ty)| format!("    pub {id}: {ty},\n"))
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: a `record:{alias}` value that holds a composed \
             value (docs/adr/0086).\n#[derive(Clone, Debug, PartialEq)]\n\
             #[allow(non_snake_case)]\npub struct {ty} {{\n{members}}}"
        )
    }
    // Read out of the saved record `value` the template's `from_saved` is
    // handed, named `what`, as `record_field` reads every other field
    // ([`rust_saved_read`]).
    fn saved_field_read(&self, ty: &ValueType, id: &str) -> Option<String> {
        let key = crate::filters::escape_rust(id.to_string());
        rust_saved_read(
            ty,
            &format!("{RUST_SAVED}::record_member(value, what, \"{key}\")?"),
            &format!("&format!(\"{{what}}.{key}\")"),
            0,
        )
    }

    fn read_binding(&self) -> &'static str {
        "v"
    }
    fn read_scalar(&self, ty: &SceType, value: &str) -> String {
        match ty {
            SceType::Uint64 => format!("{value}.unsigned64()"),
            SceType::Int64 => format!("{value}.signed64()"),
            SceType::Uint8 | SceType::Uint16 | SceType::Uint32 => format!(
                "{value}.unsigned::<{}>()",
                crate::forge::generator::rust_type(ty)
            ),
            SceType::Int8 | SceType::Int16 | SceType::Int32 => format!(
                "{value}.signed::<{}>()",
                crate::forge::generator::rust_type(ty)
            ),
            SceType::Float32 => format!("{value}.float32()"),
            SceType::Float64 => format!("{value}.float64()"),
            SceType::Bool => format!("{value}.truth()"),
            SceType::String | SceType::Bytes | SceType::Enum(_) => unreachable!(
                "a text, a byte string and an enum are cases of their own in the walk, never \
                 a scalar of a value type"
            ),
        }
    }
    fn read_enum(&self, held: &MachineEnum, value: &str) -> String {
        let arms: String = held
            .variants
            .iter()
            .map(|(declared, ident)| {
                format!(
                    "\"{}\" => Ok({}::{ident}), ",
                    crate::filters::escape_rust(declared.clone()),
                    held.ty
                )
            })
            .collect();
        format!(
            "{value}.name().and_then(|name| match name {{ {arms}other => \
             Err({value}.unknown_variant(other, \"{}\")) }})",
            crate::filters::escape_rust(held.alias.clone())
        )
    }
    // The runtime's reader answers the portable `SceString<N>` itself, the one
    // text type (docs/adr/0086, section 7.4).
    fn read_text(&self, max: u32, value: &str) -> String {
        format!("{value}.text::<{max}>()")
    }
    // The runtime's reader answers the portable `SceBytes<N>` itself.
    fn read_bytes(&self, max: u32, value: &str) -> String {
        format!("{value}.bytes::<{max}>()")
    }
    // A Rust closure's types are inferred, so the element's type is not
    // spelled.
    fn read_optional(&self, value: &str, elem: &str, _elem_ty: &str) -> String {
        format!("{value}.optional(|v| {elem})")
    }
    // The runtime's reader answers the `BoundedList<T, N>` the machine holds;
    // a lower count is held by the reader too, since no Rust type holds it
    // (docs/adr/0086, section 15.6).
    fn read_list(&self, value: &str, least: u32, max: u32, elem: &str, _elem_ty: &str) -> String {
        if least == 0 {
            format!("{value}.list::<_, {max}>(|v| {elem})")
        } else {
            format!("{value}.list_within::<_, {max}>({least}, |v| {elem})")
        }
    }
    fn read_record(
        &self,
        value: &str,
        defaults: Option<&str>,
        ty: &str,
        fields: &[(String, String)],
    ) -> String {
        let defaults = defaults.map_or_else(
            || "None".to_string(),
            |json| {
                format!(
                    "Some(\"{}\")",
                    crate::filters::escape_rust(json.to_string())
                )
            },
        );
        let members: Vec<String> = fields
            .iter()
            .map(|(id, read)| format!("{id}: r.field(\"{id}\").and_then(|v| {read})?"))
            .collect();
        format!(
            "{value}.record({defaults}, |r| Ok({ty} {{ {} }}))",
            members.join(", ")
        )
    }

    fn write_binding(&self, _depth: usize) -> String {
        "h".to_string()
    }
    fn write_scalar(&self, ty: &SceType, held: &Held) -> String {
        match (self.carrier, ty) {
            // A 64-bit integer crosses as the text of its digits.
            (RustCarrier::Json, SceType::Uint64 | SceType::Int64) => {
                format!("{RUST_PAYLOAD}::quote(&{}.to_string())", held.text)
            }
            // A number other than a 64-bit integer, and a truth value, as
            // written.
            (RustCarrier::Json, _) => format!("{}.to_string()", held.text),
            // As a scalar param is: a narrow integer widened to `i64` and a
            // `float` to `f64`, both exactly.
            (RustCarrier::ScriptValue, SceType::Uint64 | SceType::Int64) => {
                format!("{RUST_SCRIPT_VALUE}::String({}.to_string())", held.text)
            }
            (RustCarrier::ScriptValue, SceType::Bool) => {
                format!("{RUST_SCRIPT_VALUE}::Bool({})", rust_copied(held))
            }
            (RustCarrier::ScriptValue, SceType::Float32) => {
                format!(
                    "{RUST_SCRIPT_VALUE}::Double(f64::from({}))",
                    rust_copied(held)
                )
            }
            (RustCarrier::ScriptValue, SceType::Float64) => {
                format!("{RUST_SCRIPT_VALUE}::Double({})", rust_copied(held))
            }
            (RustCarrier::ScriptValue, _) => {
                format!("{RUST_SCRIPT_VALUE}::Int(i64::from({}))", rust_copied(held))
            }
        }
    }
    // The machine's enum answers the name its document declares
    // (`StaticTarget::enum_def`'s `sce_name`).
    fn write_enum(&self, held: &Held) -> String {
        match self.carrier {
            RustCarrier::Json => format!("{RUST_PAYLOAD}::quote({}.sce_name())", held.text),
            RustCarrier::ScriptValue => format!(
                "{RUST_SCRIPT_VALUE}::String({}.sce_name().to_string())",
                held.text
            ),
        }
    }
    fn write_text(&self, held: &Held) -> String {
        match self.carrier {
            RustCarrier::Json => format!("{RUST_PAYLOAD}::quote({})", held.borrowed()),
            RustCarrier::ScriptValue => {
                format!("{RUST_SCRIPT_VALUE}::String({}.to_string())", held.text)
            }
        }
    }
    // JSON has no byte string: it crosses as its byte-exact Latin-1 text.
    fn write_bytes(&self, held: &Held) -> String {
        match self.carrier {
            RustCarrier::Json => format!(
                "{RUST_PAYLOAD}::quote(&{RUST_PAYLOAD}::bytes_as_payload_text({}))",
                held.borrowed()
            ),
            RustCarrier::ScriptValue => format!(
                "{RUST_SCRIPT_VALUE}::String({RUST_PAYLOAD}::bytes_as_payload_text({}))",
                held.borrowed()
            ),
        }
    }
    fn write_optional(&self, held: &Held, elem: &str) -> String {
        match self.carrier {
            RustCarrier::Json => format!(
                "{RUST_PAYLOAD}::optional_json({}.as_ref(), |h| {elem})",
                held.text
            ),
            // `null` where the value is absent, as an absent scalar param is.
            RustCarrier::ScriptValue => format!(
                "{}.as_ref().map_or({RUST_SCRIPT_VALUE}::Null, |h| {elem})",
                held.text
            ),
        }
    }
    fn write_list(&self, held: &Held, elem: &str) -> String {
        match self.carrier {
            RustCarrier::Json => {
                format!("{RUST_PAYLOAD}::list_json({}, |h| {elem})", held.borrowed())
            }
            RustCarrier::ScriptValue => format!(
                "{RUST_SCRIPT_VALUE}::Array({}.iter().map(|h| {elem}).collect())",
                held.text
            ),
        }
    }
    fn write_field(&self, held: &Held, field: &str) -> String {
        format!("{}.{field}", held.text)
    }
    fn write_record(&self, fields: &[(String, String)]) -> String {
        match self.carrier {
            RustCarrier::Json => {
                let members: Vec<String> = fields
                    .iter()
                    .map(|(id, json)| format!("(\"{id}\", {json})"))
                    .collect();
                format!("{RUST_PAYLOAD}::record_json(vec![{}])", members.join(", "))
            }
            RustCarrier::ScriptValue => rust_script_object(fields),
        }
    }
    // The count held first, through the `AlgorithmError` the closure a `<send>`
    // evaluates a param that can fail in answers, then the list written.
    fn send_counted(&self, held: &Held, _field: &str, least: u32, sent: String) -> Option<String> {
        Some(format!(
            "{{ ::sce_forge_runtime::algorithm::holds_at_least({}.len(), {least})?; {sent} }}",
            held.text
        ))
    }

    fn copy_binding(&self) -> &'static str {
        "v"
    }
    fn copy_whole(&self, held: &Held, plain: bool) -> String {
        match (plain, held.by_reference) {
            (true, true) => format!("*{}", held.text),
            (true, false) => held.text.clone(),
            (false, _) => format!("{}.clone()", held.text),
        }
    }
    // The fit was proven, and the widening asserts it again by its const
    // parameters, so a copy the build did not prove does not compile.
    fn copy_text(&self, max: u32, held: &Held) -> String {
        format!(
            "{RUST_PORTABLE}::SceString::<{max}>::widened({})",
            held.borrowed()
        )
    }
    fn copy_bytes(&self, max: u32, held: &Held) -> String {
        format!(
            "{RUST_PORTABLE}::SceBytes::<{max}>::widened({})",
            held.borrowed()
        )
    }
    fn copy_optional(&self, held: &Held, elem: &str, _elem_ty: &str) -> String {
        format!("{}.as_ref().map(|v| {elem})", held.text)
    }
    fn copy_present(&self, value: &str) -> String {
        format!("Some({value})")
    }
    // Each element copied into the place's element type, the list's own bound
    // widened into the place's by type; the place's type names both.
    fn copy_list(&self, held: &Held, elem: &str, _elem_ty: &str) -> String {
        format!(
            "{RUST_PORTABLE}::BoundedList::widened_map({}, |v| {elem})",
            held.borrowed()
        )
    }

    // A record's unit names its fields in snake case
    // ([`crate::forge::generator::event_schema_field_ident`]), and holds a
    // number, a `bool`, a text, a byte string, and a list or an optional value
    // of them, in the machine's own types: the portable crate's bounded
    // storage, which the unit names as `::sce_portable_bytes` and the machine
    // through [`RUST_PORTABLE`], one type, a byte string that is one entry of
    // the record among them, which [`ComposedSpelling::unit_entry_bytes`]
    // therefore copies whole.
    fn unit_field(&self, held: &Held, field: &str) -> Option<String> {
        Some(format!(
            "{}.{}",
            held.text,
            crate::forge::generator::event_schema_field_ident(
                field,
                crate::generator::Language::Rust
            )
        ))
    }
    // A `match` over every variant of the unit's enum, each arm the machine's
    // variant of the same declared name, as a value of an enum crosses a call
    // (`StaticTarget::enum_crossing`): exhaustive, so a variant either side
    // gains is a build error.
    fn unit_enum(&self, machine: &MachineEnum, document: &str, held: &Held) -> Option<String> {
        use crate::generator::Language;
        let unit = crate::forge::generator::enum_qualified_type(document, Language::Rust);
        let arms: Vec<String> = machine
            .variants
            .iter()
            .map(|(declared, ident)| {
                format!(
                    "{} => {}::{ident}",
                    crate::forge::enum_naming::variant_ref(
                        Language::Rust,
                        &unit,
                        document,
                        declared
                    ),
                    machine.ty
                )
            })
            .collect();
        Some(format!(
            "match {} {{ {} }}",
            rust_copied(held),
            arms.join(", ")
        ))
    }
    // A `match` over every case of the unit's sum, a case that holds a record
    // binding it by reference, each arm the machine's case of the same
    // variant: exhaustive, as a union crosses a call.
    fn unit_union(&self, held: &Held, document: &str, arms: &[UnitUnionArm]) -> Option<String> {
        use crate::generator::Language;
        let unit = crate::forge::generator::union_unit_type(document, Language::Rust);
        let arms: String = arms
            .iter()
            .map(|arm| {
                let case = crate::forge::generator::algorithm_union_case(
                    Language::Rust,
                    &unit,
                    &arm.variant,
                    arm.holds_record.then_some(self.copy_binding()),
                );
                format!("{case} => {}, ", arm.value)
            })
            .collect();
        Some(format!("match {} {{ {arms}}}", held.borrowed()))
    }
    // The record's struct, each field named by its id as the declaration
    // names it ([`ComposedSpelling::record_def`]).
    fn record_of(&self, ty: &str, fields: &[(String, String)]) -> Option<String> {
        let members: Vec<String> = fields
            .iter()
            .map(|(id, value)| format!("{id}: {value}"))
            .collect();
        Some(format!("{ty} {{ {} }}", members.join(", ")))
    }
    // The portable list's own empty value, its element and bound inferred from
    // the place it starts (`BoundedList<T, N>`), as a list variable's is.
    fn empty_list(&self, _ty: &str) -> Option<String> {
        Some(RUST_EMPTY_LIST.to_string())
    }

    // The machine's own sum, beside its records (docs/adr/0092, section 7, the
    // Rust row): named by the union's document, so one union is one type of
    // the machine whichever alias names it.
    fn union_type(&self, name: &str) -> String {
        format!(
            "{}{}Union",
            self.machine,
            crate::filters::to_pascal_case(name.to_string())
        )
    }
    fn union_case_ident(&self, variant: &str) -> String {
        rust_case_ident(variant)
    }
    fn union_tag_read(&self, tag: &str) -> Option<String> {
        rust_union_tag_method(tag)
    }
    fn union_case_read(&self, variant: &str) -> String {
        rust_case_read(variant)
    }
    fn union_shared_read(&self, field: &str) -> String {
        rust_shared_read(field)
    }
    // Owned and compared whole, as a record that holds a composed value is;
    // every read the narrowing proves is a method here, whose other arms are
    // `unreachable!()`, the one place they are written (as a proven optional
    // is read through the runtime's one `proven`, docs/adr/0055, section 5).
    fn union_def(&self, def: &UnionDef) -> String {
        let name = crate::filters::escape_rust(def.name.clone());
        let others = |covered: usize, variant: &str| {
            if covered < def.cases.len() {
                format!(
                    "            _ => unreachable!(\"a value of {{}} read as {{}} is in another \
                     case\", \"{name}\", \"{}\"),\n",
                    crate::filters::escape_rust(variant.to_string())
                )
            } else {
                String::new()
            }
        };
        let mut variants = String::new();
        let mut tag_arms = String::new();
        let mut proven = String::new();
        for case in &def.cases {
            let ident = &case.ident;
            match &case.record {
                Some(record) => {
                    variants.push_str(&format!("    {ident}({record}),\n"));
                    tag_arms.push_str(&format!(
                        "            Self::{ident}(_) => {}::{},\n",
                        def.tag_ty, case.tag_ident
                    ));
                    proven.push_str(&format!(
                        "\n    pub fn {}(&self) -> &{record} {{\n        match self {{\n            \
                         Self::{ident}(record) => record,\n{}        }}\n    }}\n",
                        case.proven,
                        others(1, &case.variant)
                    ));
                    // The place a field of the case is written through where
                    // the case is proven (docs/adr/0092, section 6.1 item 6).
                    proven.push_str(&format!(
                        "\n    pub fn {}(&mut self) -> &mut {record} {{\n        match self {{\n            \
                         Self::{ident}(record) => record,\n{}        }}\n    }}\n",
                        rust_union_place_of(&case.proven),
                        others(1, &case.variant)
                    ));
                }
                None => {
                    variants.push_str(&format!("    {ident},\n"));
                    tag_arms.push_str(&format!(
                        "            Self::{ident} => {}::{},\n",
                        def.tag_ty, case.tag_ident
                    ));
                }
            }
        }
        let shared: String = def
            .shared
            .iter()
            .map(|field| {
                let arms: String = field
                    .cases
                    .iter()
                    .map(|ident| {
                        format!("            Self::{ident}(record) => record.{},\n", field.field)
                    })
                    .collect();
                let places: String = field
                    .cases
                    .iter()
                    .map(|ident| {
                        format!(
                            "            Self::{ident}(record) => &mut record.{},\n",
                            field.field
                        )
                    })
                    .collect();
                format!(
                    "\n    pub fn {}(&self) -> {} {{\n        match self {{\n{arms}{}        }}\n    \
                     }}\n\n    pub fn {}(&mut self) -> &mut {} {{\n        match self \
                     {{\n{places}{}        }}\n    }}\n",
                    field.accessor,
                    field.ty,
                    others(field.cases.len(), &field.field),
                    rust_union_place_of(&field.accessor),
                    field.ty,
                    others(field.cases.len(), &field.field)
                )
            })
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: a `union:{name}` value (docs/adr/0092).\n\
             #[derive(Clone, Debug, PartialEq)]\npub enum {ty} {{\n{variants}}}\n\n\
             #[allow(dead_code, non_snake_case)]\nimpl {ty} {{\n    pub fn {tag}(&self) -> {tag_ty} \
             {{\n        match self {{\n{tag_arms}        }}\n    }}\n{proven}{shared}}}",
            ty = def.ty,
            tag = def.tag_method,
            tag_ty = def.tag_ty,
        )
    }
    // The case's constructor, holding the record or nothing.
    fn union_value(&self, ty: &str, _tag: &str, case: &str, record: Option<&str>) -> String {
        match record {
            Some(record) => format!("{ty}::{case}({record})"),
            None => format!("{ty}::{case}"),
        }
    }
    // The tag first, by the enum's reader, then the record of the case it
    // names from the same object (docs/adr/0092, section 4), each case a
    // match arm the compiler holds to every variant of the tag's enum.
    fn read_union(&self, value: &str, read: &UnionRead<'_>) -> String {
        let arms: String = read
            .arms
            .iter()
            .map(|arm| {
                let case = format!("{}::{}", read.ty, arm.case_ident);
                let value = match &arm.record {
                    Some(record) => format!("{record}.map({case})"),
                    None => format!("Ok({case})"),
                };
                format!("{}::{} => {value}, ", read.tag_ty, arm.tag_ident)
            })
            .collect();
        format!(
            "{value}.union(\"{}\", |v| {}, |sce_case, v| match sce_case {{ {arms}}})",
            crate::filters::escape_rust(read.tag.to_string()),
            read.tag_reader
        )
    }
    // One object, the tag among the fields of the case (docs/adr/0092,
    // section 2.1), keys ascending as the runtime's `union_json` writes them.
    fn write_union(&self, held: &Held, ty: &str, tag: &str, arms: &[UnionWriteArm]) -> String {
        if self.carrier == RustCarrier::ScriptValue {
            return rust_script_union(held, ty, tag, arms);
        }
        let tag = crate::filters::escape_rust(tag.to_string());
        let arms: String = arms
            .iter()
            .map(|arm| {
                let variant = crate::filters::escape_rust(arm.variant.clone());
                match &arm.fields {
                    Some(fields) => {
                        let members: Vec<String> = fields
                            .iter()
                            .map(|(id, json)| format!("(\"{id}\", {json})"))
                            .collect();
                        format!(
                            "{ty}::{}(h) => {RUST_PAYLOAD}::union_json(\"{tag}\", \"{variant}\", \
                             vec![{}]), ",
                            arm.case_ident,
                            members.join(", ")
                        )
                    }
                    None => format!(
                        "{ty}::{} => {RUST_PAYLOAD}::union_json(\"{tag}\", \"{variant}\", \
                         Vec::new()), ",
                        arm.case_ident
                    ),
                }
            })
            .collect();
        format!("match {} {{ {arms}}}", held.borrowed())
    }
    fn union_member(
        &self,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        rust_union_member(union, left, member)
    }
    fn union_place_of(&self, read: &str) -> String {
        rust_union_place_of(read)
    }
    fn union_member_place(
        &self,
        holder: &str,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        rust_union_member_place(holder, union, left, member)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    const RUST: RustComposed<'static> = RustComposed {
        machine: "M",
        carrier: RustCarrier::Json,
    };

    /// The Rust row of the one table of what each backend lowers of a
    /// composed value: a payload read at any depth, a list taken whole, a
    /// composed value sent (docs/adr/0086, sections 5 and 6), a union held
    /// (docs/adr/0092, section 7), a map or a set variable (docs/adr/0108,
    /// "Order of work", step 5), and a map of a nested value (D12).
    #[test]
    fn the_rust_row_lowers_every_composed_position_and_holds_a_union() {
        use crate::forge::event_schema_check::ComposedLowering;
        assert_eq!(
            ComposedLowering::of(crate::generator::Language::Rust),
            ComposedLowering {
                name: "rust",
                reads: true,
                copies: true,
                unions: true,
                records: true,
                sends: true,
                maps: true,
                map_values: true,
                text_lists: true,
            }
        );
    }

    /// A `record:R` variable of a record that holds a composition starts at the
    /// record's struct built from its fields, a list of no lower count at the
    /// portable list's empty value (docs/adr/0087, "Decided while landing item
    /// 8", decision 10).
    #[test]
    fn a_record_is_built_from_its_fields_and_a_list_starts_empty() {
        assert_eq!(
            RUST.record_of(
                "MBoardRecord",
                &[
                    ("title".to_string(), "t".to_string()),
                    ("rects".to_string(), "l".to_string()),
                ],
            )
            .as_deref(),
            Some("MBoardRecord { title: t, rects: l }")
        );
        assert_eq!(
            RUST.empty_list(&RUST.list_type("MRectRecord", 3))
                .as_deref(),
            Some("::sce_rust_runtime::portable::BoundedList::new()")
        );
    }

    #[test]
    fn the_machine_holds_each_constructor_in_its_own_type_at_any_depth() {
        let held = held();
        let enums = enums();
        let spelled =
            |text: &str| machine_type(&ty(text), None, &held, &enums, &RUST).expect("a type");
        assert_eq!(spelled("uint16"), "u16");
        assert_eq!(
            spelled("string[8]"),
            "::sce_rust_runtime::portable::SceString<8>"
        );
        assert_eq!(
            spelled("bytes[4]"),
            "::sce_rust_runtime::portable::SceBytes<4>"
        );
        assert_eq!(spelled("optional<int64>"), "Option<i64>");
        let list = "::sce_rust_runtime::portable::BoundedList";
        assert_eq!(
            spelled("list<list<uint8>[2]>[3]"),
            format!("{list}<{list}<u8, 2>, 3>")
        );
        assert_eq!(
            spelled("list<enum:Level>[2]"),
            format!("{list}<MLevelEnum, 2>")
        );
        assert_eq!(
            spelled("list<record:Item>[3]"),
            format!("{list}<MItemRecord, 3>")
        );
        assert_eq!(
            spelled("optional<list<uint8>[2]>"),
            format!("Option<{list}<u8, 2>>")
        );
        assert!(machine_type(&ty("enum:Other"), None, &held, &enums, &RUST).is_err());
    }

    /// An algorithm's union is its unit's type (docs/adr/0092, Landing order
    /// step 5): the tag by its method, the field of the one case left by the
    /// runtime's proven read of that case's view, a field several cases declare
    /// by a `match` over them whose other arm, never taken, is the runtime's,
    /// and no read where no case left declares the field. No read spells a
    /// panic of its own.
    #[test]
    fn an_algorithms_union_is_read_on_its_units_type() {
        let shapes = shapes();
        let union = &shapes.union;
        let case = |name: &str| {
            union
                .cases
                .iter()
                .find(|c| c.variant.name == name)
                .expect("a case")
        };
        let read = |left: &[&ScopedCase], member: &str| {
            rust_unit_union_read("shape", "shape::Shape", union, left, member)
        };
        assert_eq!(read(&[], "kind"), Ok("shape.kind()".to_string()));
        assert_eq!(
            read(&[case("circle")], "radius"),
            Ok("sce_forge_runtime::union_case::proven(shape.as_circle()).radius".to_string())
        );
        assert_eq!(
            read(&[case("circle"), case("disc")], "radius"),
            Ok("sce_forge_runtime::union_case::proven(match &shape { \
                shape::Shape::Circle(record) => Some(record.radius), \
                shape::Shape::Disc(record) => Some(record.radius), _ => None, })"
                .to_string())
        );
        // Where every case of the union is left, each is an arm and there is
        // no other.
        let mut both = union.clone();
        both.cases.retain(|c| c.variant.name != "dot");
        let every: Vec<&ScopedCase> = both.cases.iter().collect();
        let text = rust_unit_union_read("shape", "shape::Shape", &both, &every, "radius")
            .expect("a read of a field every case declares");
        assert!(
            text.ends_with("Disc(record) => record.radius, })")
                && !text.contains("_ =>")
                && !text.contains("union_case"),
            "{text}"
        );
        assert!(read(&[], "radius").is_err());
        assert!(read(&[case("dot"), case("circle")], "radius").is_err());
    }

    /// A union is the machine's own sum at any depth, named by its document
    /// (docs/adr/0092, section 7), and reaches its tag's enum, then the record
    /// each case holds once, then itself.
    #[test]
    fn the_machine_holds_a_union_in_its_own_sum_after_its_cases_records() {
        let shapes = shapes();
        let kinds = kinds();
        let list = "::sce_rust_runtime::portable::BoundedList";
        assert_eq!(
            machine_type(&ty("union:Shape"), None, &shapes, &kinds, &RUST),
            Ok("MShapeUnion".to_string())
        );
        assert_eq!(
            machine_type(
                &ty("list<optional<union:Shape>>[3]"),
                None,
                &shapes,
                &kinds,
                &RUST
            ),
            Ok(format!("{list}<Option<MShapeUnion>, 3>"))
        );
        let mut reached = Reached::default();
        held_records(&ty("list<union:Shape>[2]"), None, &shapes, &mut reached).expect("reached");
        held_records(&ty("union:Shape"), None, &shapes, &mut reached).expect("reached");
        assert_eq!(reached.records, vec!["Radius".to_string()]);
        assert_eq!(reached.unions, vec![SHAPE_KEY.to_string()]);
        assert_eq!(reached.enums.into_iter().collect::<Vec<_>>(), vec!["Kind"]);
    }

    /// The declaration: one variant per case, the tag read as the machine's
    /// enum, a proven read per case that holds a record whose other arms are
    /// `unreachable!()`, and one accessor for `radius`, which `circle` and
    /// `disc` share.
    #[test]
    fn a_union_is_declared_with_its_tag_its_proven_reads_and_its_shared_fields() {
        let def = union_definition(SHAPE_KEY, &shapes(), &kinds(), &RUST).expect("declared");
        assert_eq!(def.ty, "MShapeUnion");
        assert_eq!(def.tag_method, "kind");
        assert_eq!(
            def.shared,
            vec![SharedFieldDef {
                field: "radius".to_string(),
                accessor: "proven_field_radius".to_string(),
                ty: "u16".to_string(),
                cases: vec!["Circle".to_string(), "Disc".to_string()],
            }]
        );
        let text = RUST.union_def(&def);
        for fragment in [
            "#[derive(Clone, Debug, PartialEq)]\npub enum MShapeUnion {\n    Dot,\n    \
             Circle(MRadiusRecord),\n    Disc(MRadiusRecord),\n}",
            "    pub fn kind(&self) -> MKindEnum {\n",
            "            Self::Dot => MKindEnum::Dot,\n",
            "            Self::Circle(_) => MKindEnum::Circle,\n",
            "    pub fn proven_circle(&self) -> &MRadiusRecord {\n",
            "            Self::Circle(record) => record,\n",
            "    pub fn proven_field_radius(&self) -> u16 {\n",
            "            Self::Disc(record) => record.radius,\n",
            // ... and the place each is written through (section 6.1 item 6).
            "    pub fn proven_circle_mut(&mut self) -> &mut MRadiusRecord {\n",
            "    pub fn proven_field_radius_mut(&mut self) -> &mut u16 {\n",
            "            Self::Disc(record) => &mut record.radius,\n",
        ] {
            assert!(text.contains(fragment), "expected `{fragment}` in:\n{text}");
        }
        assert!(!text.contains("proven_dot"), "{text}");
        assert_eq!(text.matches("unreachable!(").count(), 6, "{text}");
    }

    /// A write of a union's field (docs/adr/0092, section 6.1 item 6): through
    /// the mutable proven read of the one case left, or the place of a field
    /// two cases share; the tag is never written alone.
    #[test]
    fn a_proven_write_is_spelled_by_the_cases_left() {
        let shapes = shapes();
        let union = &shapes.union;
        let case = |name: &str| {
            union
                .cases
                .iter()
                .find(|case| case.variant.name == name)
                .expect("a case")
        };
        let all: Vec<&ScopedCase> = union.cases.iter().collect();
        assert_eq!(
            rust_union_member_place("self.s", union, &[case("disc")], "radius"),
            Ok("self.s.proven_disc_mut().radius".into())
        );
        assert_eq!(
            rust_union_member_place("self.s", union, &[case("circle"), case("disc")], "radius"),
            Ok("*self.s.proven_field_radius_mut()".into())
        );
        assert!(rust_union_member_place("self.s", union, &all, "kind")
            .unwrap_err()
            .contains("never written alone"));
        assert!(rust_union_member_place("self.s", union, &[case("dot")], "radius").is_err());
    }

    /// The reader reads the tag by the enum's reader and then the record of
    /// the case it names; the writer writes the tag among the case's fields.
    #[test]
    fn a_union_is_read_tag_first_and_written_by_its_case() {
        let shapes = shapes();
        let read = reader(&ty("union:Shape"), None, &shapes, &kinds(), &RUST, "v").expect("read");
        assert!(
            read.starts_with(
                "v.union(\"kind\", |v| v.name().and_then(|name| match name { \"dot\" \
                              => Ok(MKindEnum::Dot), "
            ),
            "{read}"
        );
        assert!(
            read.contains(
                "|sce_case, v| match sce_case { MKindEnum::Dot => Ok(MShapeUnion::Dot), \
                 MKindEnum::Circle => v.record(None, |r| Ok(MRadiusRecord { radius: \
                 r.field(\"radius\").and_then(|v| v.unsigned::<u16>())? })).map(MShapeUnion::Circle), "
            ),
            "{read}"
        );
        let p = "::sce_rust_runtime::event_payload";
        assert_eq!(
            writer(
                &ty("union:Shape"),
                None,
                &shapes,
                &RUST,
                &Held::place("x.one")
            ),
            Ok(format!(
                "match &x.one {{ MShapeUnion::Dot => {p}::union_json(\"kind\", \"dot\", \
                 Vec::new()), MShapeUnion::Circle(h) => {p}::union_json(\"kind\", \"circle\", \
                 vec![(\"radius\", h.radius.to_string())]), MShapeUnion::Disc(h) => \
                 {p}::union_json(\"kind\", \"disc\", vec![(\"radius\", h.radius.to_string())]), }}"
            ))
        );
    }

    /// A read of a union's member (docs/adr/0092, section 6.1): the tag by its
    /// method, a field of the one case left through that case's proven read,
    /// and a field two cases share through its accessor.
    #[test]
    fn a_proven_read_is_spelled_by_the_cases_left() {
        let shapes = shapes();
        let union = &shapes.union;
        let case = |name: &str| {
            union
                .cases
                .iter()
                .find(|case| case.variant.name == name)
                .expect("a case")
        };
        let all: Vec<&ScopedCase> = union.cases.iter().collect();
        assert_eq!(rust_union_member(union, &all, "kind"), Ok("kind()".into()));
        assert_eq!(
            rust_union_member(union, &[case("disc")], "radius"),
            Ok("proven_disc().radius".into())
        );
        assert_eq!(
            rust_union_member(union, &[case("circle"), case("disc")], "radius"),
            Ok("proven_field_radius()".into())
        );
        assert!(rust_union_member(union, &[case("dot")], "radius").is_err());
    }

    #[test]
    fn a_record_is_declared_with_each_field_in_its_machine_type() {
        let held = held();
        let members = record_members("Item", &held, &enums(), &RUST).expect("members");
        assert_eq!(
            members,
            vec![
                ("id".to_string(), "u32".to_string()),
                ("level".to_string(), "MLevelEnum".to_string()),
                ("due".to_string(), "Option<i64>".to_string()),
                (
                    "blob".to_string(),
                    "::sce_rust_runtime::portable::SceBytes<4>".to_string()
                ),
                (
                    "tags".to_string(),
                    "::sce_rust_runtime::portable::BoundedList<::sce_rust_runtime::portable::SceString<8>, 4>"
                        .to_string()
                ),
            ]
        );
        // Every bound is held by a member's type, so a Rust record checks
        // nothing where it is made.
        let checks = record_checks("Item", &held, &RUST).expect("checks");
        assert!(checks.is_empty(), "{checks:?}");
        let def = RUST.record_def("MItemRecord", "Item", &members, &checks);
        assert!(
            def.contains("pub struct MItemRecord {\n    pub id: u32,\n"),
            "{def}"
        );
        assert!(
            def.contains(
                "    pub tags: ::sce_rust_runtime::portable::BoundedList<::sce_rust_runtime::portable::SceString<8>, \
                 4>,\n}"
            ),
            "{def}"
        );
    }

    /// A Rust record holds every bound in a member's type, a record whose
    /// fields hold records and lists of lists included, so it checks nothing
    /// where it is made.
    #[test]
    fn a_rust_record_checks_no_bound_where_it_is_made() {
        let held = held();
        assert!(record_checks("Batch", &held, &RUST)
            .expect("checks")
            .is_empty());
    }

    #[test]
    fn a_reader_is_one_runtime_call_per_constructor() {
        let held = held();
        let enums = enums();
        let read = |text: &str| reader(&ty(text), None, &held, &enums, &RUST, "v").expect("read");
        assert_eq!(read("uint8"), "v.unsigned::<u8>()");
        assert_eq!(read("int32"), "v.signed::<i32>()");
        assert_eq!(read("uint64"), "v.unsigned64()");
        assert_eq!(read("int64"), "v.signed64()");
        assert_eq!(read("float32"), "v.float32()");
        assert_eq!(read("bool"), "v.truth()");
        assert_eq!(read("string[8]"), "v.text::<8>()");
        assert_eq!(read("bytes[4]"), "v.bytes::<4>()");
        assert_eq!(read("optional<int64>"), "v.optional(|v| v.signed64())");
        assert_eq!(
            read("list<list<uint8>[2]>[3]"),
            "v.list::<_, 3>(|v| v.list::<_, 2>(|v| v.unsigned::<u8>()))"
        );
        // A lower count is handed to the reader, at any depth (docs/adr/0086,
        // section 15.5); the machine's type is the one its bound gives.
        assert_eq!(
            read("list<list<uint8>[1..2]>[2..3]"),
            "v.list_within::<_, 3>(2, |v| v.list_within::<_, 2>(1, |v| v.unsigned::<u8>()))"
        );
        assert_eq!(
            machine_type(&ty("list<uint8>[2..3]"), None, &held, &enums, &RUST),
            machine_type(&ty("list<uint8>[3]"), None, &held, &enums, &RUST)
        );
        assert_eq!(
            read("enum:Level"),
            "v.name().and_then(|name| match name { \"low\" => Ok(MLevelEnum::Low), \"high\" => \
             Ok(MLevelEnum::High), other => Err(v.unknown_variant(other, \"Level\")) })"
        );
    }

    #[test]
    fn a_record_is_read_field_by_field_with_its_defaults() {
        let held = held();
        let enums = enums();
        let item = reader(&ty("record:Item"), None, &held, &enums, &RUST, "v").expect("read");
        assert!(
            item.starts_with("v.record(None, |r| Ok(MItemRecord { id: r.field(\"id\").and_then(|v| v.unsigned::<u32>())?, "),
            "{item}"
        );
        assert!(
            item.ends_with(
                "tags: r.field(\"tags\").and_then(|v| v.list::<_, 4>(|v| v.text::<8>()))? }))"
            ),
            "{item}"
        );
        // A list the second revision added reads empty where a delivery lacks it.
        let batch = reader(&ty("record:Batch"), None, &held, &enums, &RUST, "v").expect("read");
        assert!(
            batch.starts_with("v.record(Some(\"{\\\"late\\\":[]}\"), |r| Ok(MBatchRecord {"),
            "{batch}"
        );
    }

    #[test]
    fn a_value_that_fits_is_copied_by_the_case_of_each_constructor() {
        let held = held();
        let enums = enums();
        let copy = |from: &str, to: &str, at: Held| {
            copier(&ty(from), None, &ty(to), None, &held, &enums, &RUST, &at)
        };
        // One type in the machine: the value as it is.
        assert_eq!(
            copy("list<uint8>[4]", "list<uint8>[4]", Held::place("x")),
            Ok("x.clone()".to_string())
        );
        // A list of a smaller bound is widened by type, each element copied.
        assert_eq!(
            copy("list<uint8>[2]", "list<uint8>[4]", Held::place("x")),
            Ok("::sce_rust_runtime::portable::BoundedList::widened_map(&x, |v| *v)".to_string())
        );
        assert_eq!(
            copy(
                "list<record:Item>[3]",
                "list<record:Item>[3]",
                Held::place("x")
            ),
            Ok("x.clone()".to_string())
        );
        assert_eq!(
            copy("optional<int64>", "optional<int64>", Held::reference("v")),
            Ok("*v".to_string())
        );
        // A text of a smaller bound, at depth 1 and 2.
        assert_eq!(
            copy("string[4]", "string[8]", Held::place("x")),
            Ok("::sce_rust_runtime::portable::SceString::<8>::widened(&x)".to_string())
        );
        assert_eq!(
            copy("list<string[4]>[2]", "list<string[8]>[3]", Held::place("x")),
            Ok(
                "::sce_rust_runtime::portable::BoundedList::widened_map(&x, |v| \
                 ::sce_rust_runtime::portable::SceString::<8>::widened(v))"
                    .to_string()
            )
        );
        assert_eq!(
            copy("string[4]", "optional<string[8]>", Held::place("x")),
            Ok("Some(::sce_rust_runtime::portable::SceString::<8>::widened(&x))".to_string())
        );
        // A byte string of a smaller bound is widened by type too.
        assert_eq!(
            copy("bytes[2]", "bytes[4]", Held::place("x")),
            Ok("::sce_rust_runtime::portable::SceBytes::<4>::widened(&x)".to_string())
        );
        // A pair of types the walk has no copy of is refused.
        assert!(copy("record:Item", "list<record:Item>[3]", Held::place("x")).is_err());
    }

    #[test]
    fn a_writer_is_one_runtime_call_per_constructor() {
        let held = held();
        let write =
            |text: &str, at: Held| writer(&ty(text), None, &held, &RUST, &at).expect("write");
        let p = "::sce_rust_runtime::event_payload";
        assert_eq!(write("uint8", Held::place("x.n")), "x.n.to_string()");
        assert_eq!(
            write("int64", Held::place("x.n")),
            format!("{p}::quote(&x.n.to_string())")
        );
        assert_eq!(
            write("string[8]", Held::place("x.s")),
            format!("{p}::quote(&x.s)")
        );
        assert_eq!(
            write("string[8]", Held::reference("h")),
            format!("{p}::quote(h)")
        );
        assert_eq!(
            write("enum:Level", Held::reference("h")),
            format!("{p}::quote(h.sce_name())")
        );
        assert_eq!(
            write("list<optional<uint16>>[2]", Held::place("x.maybe")),
            format!(
                "{p}::list_json(&x.maybe, |h| {p}::optional_json(h.as_ref(), |h| h.to_string()))"
            )
        );
        assert_eq!(
            write("list<record:Item>[3]", Held::place("x.items")),
            format!(
                "{p}::list_json(&x.items, |h| {p}::record_json(vec![(\"id\", h.id.to_string()), \
                 (\"level\", {p}::quote(h.level.sce_name())), (\"due\", {p}::optional_json(h.due.as_ref(), \
                 |h| {p}::quote(&h.to_string()))), (\"blob\", {p}::quote(&{p}::bytes_as_payload_text(&h.blob))), \
                 (\"tags\", {p}::list_json(&h.tags, |h| {p}::quote(h)))]))"
            )
        );
    }

    /// A list sent where its field asks a lower count it does not prove is
    /// held to the count before it is written (docs/adr/0086, section 15.6).
    #[test]
    fn a_list_sent_short_of_its_fields_count_fails_before_it_is_written() {
        assert_eq!(
            RUST.send_counted(&Held::place("self.taking"), "ops", 1, "W".to_string()),
            Some(
                "{ ::sce_forge_runtime::algorithm::holds_at_least(self.taking.len(), 1)?; W }"
                    .to_string()
            )
        );
    }

    /// A list that states a lower count is read back from a saved state held
    /// to it as well as to its bound, which its `BoundedList` holds alone, so
    /// a restore refuses a list a delivery would (docs/adr/0086, section
    /// 15.6); a value that holds one by an element or a present value is read
    /// through it; a type that states none, and a record a list holds, are
    /// left to their own `SavedValue`.
    #[test]
    fn a_rust_saved_list_is_held_to_its_lower_count() {
        let s = "::sce_rust_runtime::saved_state";
        let read = |text: &str| rust_saved_read(&ty(text), "v", "w", 0);
        assert_eq!(
            read("list<record:Item>[1..3]"),
            Some(format!(
                "{s}::bounded_within::<_, 3>(v, w, 1, {s}::SavedValue::from_saved)"
            ))
        );
        assert_eq!(
            read("optional<list<list<uint8>[1..2]>[3]>"),
            Some(format!(
                "{s}::optional(v, |sce_present0| {s}::bounded_within::<_, 3>(sce_present0, w, 0, \
                 |sce_item1, sce_what1| {s}::bounded_within::<_, 2>(sce_item1, sce_what1, 1, \
                 {s}::SavedValue::from_saved)))"
            ))
        );
        assert_eq!(read("list<list<uint8>[2]>[3]"), None);
        assert_eq!(read("optional<list<uint8>[2]>"), None);
        assert_eq!(read("record:Item"), None);
        assert_eq!(
            RUST.saved_field_read(&ty("list<record:Item>[1..3]"), "items"),
            Some(format!(
                "{s}::bounded_within::<_, 3>({s}::record_member(value, what, \"items\")?, \
                 &format!(\"{{what}}.items\"), 1, {s}::SavedValue::from_saved)"
            ))
        );
        assert_eq!(RUST.saved_field_read(&ty("list<uint8>[3]"), "picked"), None);
    }

    /// docs/SCE_TEST_PROFILE.md §12.3: the machine's value of a message is
    /// made from the value its record's unit holds. Each field is read by its
    /// snake-case name on the unit's record; a list of records is mapped
    /// element by element into the machine's records, an enum by a `match`
    /// into the machine's variant of the same declared name, and every value
    /// that names no record, enum or union is the machine's own type in the
    /// unit too, copied whole: a number as it is, a text, a byte string and a
    /// list cloned, a byte string that is one entry of its record included,
    /// which the unit holds in the machine's `SceBytes` of its bound.
    #[test]
    fn a_message_is_made_from_its_units_value_field_by_field() {
        let held = held();
        let documents: BTreeMap<String, String> =
            [("Level".to_string(), "level".to_string())].into();
        let batch = held.record(None, "Batch").expect("the record");
        let fields = fields_from_unit(
            batch,
            &held,
            &enums(),
            &documents,
            &RUST,
            &Held::reference("sce_value"),
        )
        .expect("every field is made");
        let item = "MItemRecord { id: v.id, level: match v.level { level::Level::Low => \
                    MLevelEnum::Low, level::Level::High => MLevelEnum::High }, due: v.due, \
                    blob: v.blob.clone(), tags: v.tags.clone() }";
        assert_eq!(
            fields,
            vec![
                (
                    "items".to_string(),
                    format!(
                        "::sce_rust_runtime::portable::BoundedList::widened_map(\
                         &sce_value.items, |v| {item})"
                    )
                ),
                ("grid".to_string(), "sce_value.grid.clone()".to_string()),
                ("late".to_string(), "sce_value.late.clone()".to_string()),
            ]
        );
        // An enum whose document the machine does not name has no unit to
        // make a value from.
        assert!(fields_from_unit(
            batch,
            &held,
            &enums(),
            &BTreeMap::new(),
            &RUST,
            &Held::reference("sce_value"),
        )
        .unwrap_err()
        .contains("`enum:Level` names an enum whose document the machine does not hold"));
    }

    /// docs/adr/0092, section 7: a union is made by a `match` over every case
    /// of the unit's sum, a case that holds a record binding it, each arm the
    /// machine's case of the same variant holding the record made from it; a
    /// union that may be absent is mapped where it is present.
    #[test]
    fn a_union_is_made_from_its_units_value_case_by_case() {
        let shapes = shapes();
        let made = |text: &str, held: &Held| {
            from_unit(
                &ty(text),
                None,
                &shapes,
                &kinds(),
                &BTreeMap::new(),
                &RUST,
                held,
            )
        };
        let cases = "shape::Shape::Dot => MShapeUnion::Dot, shape::Shape::Circle(v) => \
                     MShapeUnion::Circle(MRadiusRecord { radius: v.radius }), \
                     shape::Shape::Disc(v) => MShapeUnion::Disc(MRadiusRecord { radius: \
                     v.radius }), ";
        assert_eq!(
            made("union:Shape", &Held::place("sce_value.shape")),
            Ok(format!("match &sce_value.shape {{ {cases}}}"))
        );
        assert_eq!(
            made("optional<union:Shape>", &Held::place("sce_value.shape")),
            Ok(format!(
                "sce_value.shape.as_ref().map(|v| match v {{ {cases}}})"
            ))
        );
    }

    /// A spelling that makes no machine's value from a unit's (the ECMAScript
    /// one, which holds no unit of a record and answers none of the walk's
    /// reads of one) refuses the walk by name, at the first field it would
    /// read.
    #[test]
    fn a_backend_that_makes_no_value_from_a_units_refuses_by_name() {
        let js = crate::forge::composed_lowering::JsComposed;
        let held = held();
        let batch = held.record(None, "Batch").expect("the record");
        assert_eq!(
            fields_from_unit(
                batch,
                &held,
                &enums(),
                &BTreeMap::new(),
                &js,
                &Held::reference("sce_value"),
            ),
            Err(
                "the field `items` of `record:Batch` has no machine's value made from its unit's \
                 on this backend"
                    .to_string()
            )
        );
    }
}
