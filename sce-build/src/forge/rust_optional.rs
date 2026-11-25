// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! How the Rust backend spells a value that may be absent (docs/adr/0055,
//! section 5): one function per construct, which the Rust expression emitter,
//! the Rust static lowering and the Rust payload builder all call, so that an
//! optional value is written one way wherever generated Rust holds one. The
//! type itself is the backend's one spelling of it, the generator's
//! `optional_of`.
//!
//! The type is `core`'s `Option<T>`: absent is `None`, present is `Some(v)`. It
//! needs no allocator, so the bare-metal profile holds one as the standard
//! profile does.
//!
//! **Held and read.** A place that holds an optional value (a variable, a
//! record's field, a payload's field) holds it as `Option<T>`. Where an
//! expression reads one, it reads its [`view`]: the value itself for an
//! element that is `Copy` (a number, a `bool`, an enum), and `as_deref()` for a
//! text or a byte string, which a machine owns and an expression borrows as
//! `&str` or `&[u8]`, as it borrows a text that is always present. Every
//! spelling below that reads a value is handed its view; [`owned`] turns a
//! view back into the value a place holds.
//!
//! **A proven read has no failure path.** `x.value` is lowered only where the
//! build proved `x` present, through [`proven`], whose absent arm is the
//! runtime's `unreachable!` (`sce_forge_runtime::optional::proven`), written
//! once there.
//!
//! **A tree where an expression is built.** The spellings the Rust expression
//! emitter reads are built as its tree ([`RustExpr`]), whose printer groups each
//! operand where Rust's grammar needs it; a spelling the statement lowering
//! splices into text takes its operands as text, which stands as one operand
//! ([`RustExpr::atom`]), and is printed from the same tree.

use crate::forge::expr::rust_tree::{RustArm, RustBinOp, RustExpr, RustUnOp};
use crate::forge::types::OptionalElem;

/// The runtime module the spellings below call.
const RUNTIME: &str = "sce_forge_runtime::optional";

/// The name the present value is bound to inside a `match` or a closure this
/// module writes. Prefixed, as the other bindings generated Rust introduces
/// are (`sce_value`), so that it never names a value the document declares.
pub const PRESENT: &str = "sce_present";

/// The absent value.
pub const ABSENT: &str = "None";

/// A value of `T` where an optional value is expected, which is present.
pub fn present(value: &str) -> String {
    present_expr(RustExpr::atom(value)).to_string()
}

/// [`present`], of the value `value` an expression is.
pub(crate) fn present_expr(value: RustExpr) -> RustExpr {
    RustExpr::path_call("Some", vec![value])
}

/// The optional value `held` holds, as an expression reads it: `held` itself
/// for an element that is `Copy`, and `held.as_deref()` (an `Option<&str>` or
/// `Option<&[u8]>`) for a text or a byte string.
pub(crate) fn view(held: RustExpr, elem: OptionalElem) -> RustExpr {
    match elem {
        OptionalElem::Str | OptionalElem::Bytes => held.method("as_deref", Vec::new()),
        _ => held,
    }
}

/// The value a place of `optional<T>` is written from, given the [`view`] of
/// another optional value: the view itself for an element that is `Copy`, and
/// an owned copy of the borrowed text or bytes otherwise.
pub(crate) fn owned(view: RustExpr, elem: OptionalElem) -> RustExpr {
    match elem {
        OptionalElem::Str => view.method("map", vec![RustExpr::atom("str::to_string")]),
        OptionalElem::Bytes => view.method("map", vec![RustExpr::atom("<[u8]>::to_vec")]),
        _ => view,
    }
}

/// `value`, an optional value, with `map` applied to its present value — `map`
/// spells it for the bound present value, [`PRESENT`] — and absent where it is
/// absent: how a present text is held to the bound of the place it is written
/// to. A `match` rather than a closure, so that a `?` in `map` returns from the
/// statement it is written in.
pub fn mapped(value: &str, map: &dyn Fn(&str) -> String) -> String {
    format!(
        "match {value} {{ Some({PRESENT}) => Some({}), None => None }}",
        map(PRESENT)
    )
}

/// `x !== null`, of the [`view`] of `x`.
pub(crate) fn is_present(view: RustExpr) -> RustExpr {
    view.method("is_some", Vec::new())
}

/// `x === null`, of the [`view`] of `x`.
pub(crate) fn is_absent(view: RustExpr) -> RustExpr {
    view.method("is_none", Vec::new())
}

/// `x.value` where the build has proven `x` present, of the [`view`] of `x`:
/// a `T`, or the `&str` / `&[u8]` an expression reads a text or bytes as.
pub(crate) fn proven(view: RustExpr) -> RustExpr {
    RustExpr::path_call(format!("{RUNTIME}::proven"), vec![view])
}

/// `x.value` where the build has proven the optional record `held` present,
/// read as the record's own place (`&T`) and not moved out
/// (docs/adr/0055, step 9): the runtime's `proven_ref` over a borrow of `held`,
/// over which the member `.f` of the field read is spelled. A record that owns
/// text is not `Copy`, and what holds it keeps it, so [`proven`], which takes the
/// value, is for an element that is. `held` is a place or a postfix expression
/// (`self.pick`, `x`).
pub fn proven_ref(held: &str) -> String {
    proven_ref_expr(RustExpr::atom(held)).to_string()
}

/// [`proven_ref`], of the place `held` an expression reads.
pub(crate) fn proven_ref_expr(held: RustExpr) -> RustExpr {
    RustExpr::path_call(
        format!("{RUNTIME}::proven_ref"),
        vec![held.prefixed(RustUnOp::Ref)],
    )
}

/// The place `field` of the present record `held` is written through, where the
/// build has proven `held` present (docs/adr/0055, step 9): the runtime's
/// `proven_mut` over a mutable borrow of `held`, which the field names. The
/// location of an assignment, `x.value.f = e`; `held` is a place or a postfix
/// expression, as for [`proven_ref`].
pub fn proven_field_place(held: &str, field: &str) -> String {
    format!("{RUNTIME}::proven_mut(&mut {held}).{field}")
}

/// `x ?? d`, of the [`view`] of `x` and the spelling of `d`: the present value
/// of `x`, and `d` only where `x` is absent, so `d` is evaluated (and may fail)
/// only then, and `x` once. `d_is_optional` says that `d` is itself an
/// optional value, which makes the whole an optional value: the present `x`
/// is then lifted, as `d` already is one.
pub(crate) fn coalesce(view: RustExpr, default: RustExpr, d_is_optional: bool) -> RustExpr {
    let bound = RustExpr::atom(PRESENT);
    let taken = if d_is_optional {
        present_expr(bound)
    } else {
        bound
    };
    RustExpr::match_of(
        view,
        vec![
            RustArm {
                pattern: format!("Some({PRESENT})"),
                body: taken,
            },
            RustArm {
                pattern: ABSENT.to_string(),
                body: default,
            },
        ],
        false,
    )
}

/// `x === y` (or `x !== y` where `equal` is false) of two optional values, or
/// of an optional value and a [`present`] one, each given as its [`view`]:
/// total, since two absent values are equal and an absent one equals no
/// present one, which is `Option`'s own `==`.
pub(crate) fn equality(left: RustExpr, right: RustExpr, equal: bool) -> RustExpr {
    let op = if equal { RustBinOp::Eq } else { RustBinOp::Ne };
    RustExpr::binary(op, left, right)
}

/// The value of a `<param>` of a `<send>` or a `<donedata>` that carries the
/// optional value `value` (owned, as [`owned`] gives it): the runtime's `null`
/// for an absent value, and `present_wire` of the bound present value
/// ([`PRESENT`]) for a present one, as a value of `T` crosses there.
pub fn wire(value: &str, present_wire: &dyn Fn(&str) -> String) -> String {
    format!(
        "match {value} {{ Some({PRESENT}) => {}, None => ::sce_rust_runtime::ScriptValue::Null }}",
        present_wire(PRESENT)
    )
}

/// What a `<log>` prints for the optional value `value` (a place or a
/// [`view`]): its present value as a value of `T` prints, and `null` for an
/// absent one.
pub fn logged(value: &str) -> String {
    format!("{RUNTIME}::logged(&{value})")
}

/// The lift of the payload field `field` of `optional<T>` out of the
/// delivered `fields` (`::sce_rust_runtime::event_payload::PayloadFields`),
/// given `present_read`, the reader of a present value of `T` as the lift
/// spells it (`fields.signed64("due")?`): absent for `null`, the value
/// otherwise, and a delivery that lacks the key refused.
pub fn payload_read(field: &str, present_read: &str) -> String {
    format!("fields.optional(\"{field}\", |fields| Ok({present_read}))?")
}

/// The JSON text the inject seam writes for the payload field `held` of
/// `optional<T>`: `null` for an absent value, and `present_json` of the bound
/// present value (a reference, [`PRESENT`]) for a present one. The key is
/// always written. The placeholder this fills is a bare `{}`: a 64-bit
/// integer's quotes are `present_json`'s to write, since an absent one has
/// none.
pub fn payload_written(held: &str, present_json: &dyn Fn(&str) -> String) -> String {
    format!(
        "::sce_rust_runtime::event_payload::optional_json({held}.as_ref(), |{PRESENT}| {})",
        present_json(PRESENT)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::quantity::NumericBaseType;

    const INT64: OptionalElem = OptionalElem::Number(NumericBaseType::Int {
        signed: true,
        bits: 64,
    });

    /// `text` parses as one Rust expression: a spelling the emitter splices
    /// into generated code is at least well formed on its own.
    fn expression(text: &str) -> String {
        syn::parse_str::<syn::Expr>(text)
            .unwrap_or_else(|e| panic!("`{text}` is no Rust expression: {e}"));
        text.to_string()
    }

    /// The text of the expression `built`, held to be well formed as
    /// [`expression`] holds a text.
    fn printed(built: RustExpr) -> String {
        expression(&built.to_string())
    }

    /// A place or a postfix expression an expression reads.
    fn at(text: &str) -> RustExpr {
        RustExpr::atom(text)
    }

    #[test]
    fn the_two_values_are_none_and_some() {
        assert_eq!(expression(ABSENT), "None");
        assert_eq!(expression(&present("7")), "Some(7)");
    }

    #[test]
    fn a_text_or_bytes_is_read_borrowed_and_every_other_element_as_it_is() {
        assert_eq!(printed(view(at("self.due"), INT64)), "self.due");
        assert_eq!(
            printed(view(at("self.status"), OptionalElem::Enum)),
            "self.status"
        );
        assert_eq!(
            printed(view(at("self.title"), OptionalElem::Str)),
            "self.title.as_deref()"
        );
        assert_eq!(
            printed(view(at("ev.tag"), OptionalElem::Bytes)),
            "ev.tag.as_deref()"
        );
    }

    #[test]
    fn a_borrowed_view_is_owned_again_where_a_place_holds_it() {
        let title = view(at("ev.title"), OptionalElem::Str);
        assert_eq!(
            printed(owned(title, OptionalElem::Str)),
            "ev.title.as_deref().map(str::to_string)"
        );
        let tag = view(at("ev.tag"), OptionalElem::Bytes);
        assert_eq!(
            printed(owned(tag, OptionalElem::Bytes)),
            "ev.tag.as_deref().map(<[u8]>::to_vec)"
        );
        assert_eq!(printed(owned(at("ev.due"), INT64)), "ev.due");
    }

    #[test]
    fn a_present_value_is_mapped_and_an_absent_one_stays_absent() {
        assert_eq!(
            expression(&mapped("ev.title.as_deref().map(str::to_string)", &|v| {
                format!("::sce_rust_runtime::portable::SceString::<16>::from_text(&{v})?")
            })),
            "match ev.title.as_deref().map(str::to_string) { Some(sce_present) => \
             Some(::sce_rust_runtime::portable::SceString::<16>::from_text(&sce_present)?), None \
             => None }"
        );
    }

    #[test]
    fn a_test_of_presence_is_is_some_and_of_absence_is_none() {
        assert_eq!(printed(is_present(at("self.due"))), "self.due.is_some()");
        assert_eq!(
            printed(is_absent(view(at("self.title"), OptionalElem::Str))),
            "self.title.as_deref().is_none()"
        );
    }

    #[test]
    fn a_proven_read_goes_through_the_runtime() {
        assert_eq!(
            printed(proven(at("self.due"))),
            "sce_forge_runtime::optional::proven(self.due)"
        );
    }

    #[test]
    fn a_proven_record_is_read_and_written_in_place_through_the_runtime() {
        assert_eq!(
            expression(&proven_ref("self.pick")),
            "sce_forge_runtime::optional::proven_ref(&self.pick)"
        );
        // A field of it is read through the borrow ...
        assert_eq!(
            expression(&format!("{}.count", proven_ref("self.pick"))),
            "sce_forge_runtime::optional::proven_ref(&self.pick).count"
        );
        // ... and written through the mutable one, as the place of an assignment.
        assert_eq!(
            expression(&proven_field_place("self.pick", "count")),
            "sce_forge_runtime::optional::proven_mut(&mut self.pick).count"
        );
        syn::parse_str::<syn::Stmt>(&format!("{} = 7;", proven_field_place("held", "count")))
            .expect("the place is one a Rust assignment writes through");
    }

    #[test]
    fn a_default_is_evaluated_only_in_the_absent_arm() {
        let d = RustExpr::path_call(
            "sce_forge_runtime::algorithm::mul::<i64>",
            vec![at("self.huge"), at("self.huge")],
        )
        .tried();
        assert_eq!(
            printed(coalesce(at("self.due"), d, false)),
            "match self.due { Some(sce_present) => sce_present, None => \
             sce_forge_runtime::algorithm::mul::<i64>(self.huge, self.huge)? }"
        );
        assert_eq!(
            printed(coalesce(
                view(at("self.title"), OptionalElem::Str),
                at("\"none\""),
                false
            )),
            "match self.title.as_deref() { Some(sce_present) => sce_present, None => \"none\" }"
        );
        // An optional default keeps the whole an optional value.
        assert_eq!(
            printed(coalesce(at("self.due"), at("self.other"), true)),
            "match self.due { Some(sce_present) => Some(sce_present), None => self.other }"
        );
    }

    #[test]
    fn equality_is_options_own() {
        assert_eq!(
            printed(equality(at("self.due"), at("self.other"), true)),
            "self.due == self.other"
        );
        assert_eq!(
            printed(equality(
                at("self.status"),
                present_expr(at("StaticOptionalStatusEnum::Done")),
                false
            )),
            "self.status != Some(StaticOptionalStatusEnum::Done)"
        );
    }

    #[test]
    fn a_carrier_writes_an_absent_value_as_the_runtimes_null() {
        assert_eq!(
            expression(&wire("self.due", &|v| format!(
                "::sce_rust_runtime::ScriptValue::String(({v}).to_string())"
            ))),
            "match self.due { Some(sce_present) => \
             ::sce_rust_runtime::ScriptValue::String((sce_present).to_string()), \
             None => ::sce_rust_runtime::ScriptValue::Null }"
        );
        assert_eq!(
            expression(&logged("self.due")),
            "sce_forge_runtime::optional::logged(&self.due)"
        );
    }

    #[test]
    fn a_payload_reads_null_as_absent_and_writes_every_key() {
        assert_eq!(
            expression(&payload_read("due", "fields.signed64(\"due\")?")),
            "fields.optional(\"due\", |fields| Ok(fields.signed64(\"due\")?))?"
        );
        assert_eq!(
            expression(&payload_written("payload.due", &|v| format!(
                "format!(\"\\\"{{}}\\\"\", {v})"
            ))),
            "::sce_rust_runtime::event_payload::optional_json(payload.due.as_ref(), \
             |sce_present| format!(\"\\\"{}\\\"\", sce_present))"
        );
    }
}
