// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
    format!("Some({value})")
}

/// The optional value `held` holds, as an expression reads it: `held` itself
/// for an element that is `Copy`, and `held.as_deref()` (an `Option<&str>` or
/// `Option<&[u8]>`) for a text or a byte string. `held` is a place or a
/// postfix expression (`self.title`, `ev.title`).
pub fn view(held: &str, elem: OptionalElem) -> String {
    match elem {
        OptionalElem::Str | OptionalElem::Bytes => format!("{held}.as_deref()"),
        _ => held.to_string(),
    }
}

/// The value a place of `optional<T>` is written from, given the [`view`] of
/// another optional value: the view itself for an element that is `Copy`, and
/// an owned copy of the borrowed text or bytes otherwise.
pub fn owned(view: &str, elem: OptionalElem) -> String {
    match elem {
        OptionalElem::Str => format!("{view}.map(str::to_string)"),
        OptionalElem::Bytes => format!("{view}.map(<[u8]>::to_vec)"),
        _ => view.to_string(),
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
pub fn is_present(view: &str) -> String {
    format!("{view}.is_some()")
}

/// `x === null`, of the [`view`] of `x`.
pub fn is_absent(view: &str) -> String {
    format!("{view}.is_none()")
}

/// `x.value` where the build has proven `x` present, of the [`view`] of `x`:
/// a `T`, or the `&str` / `&[u8]` an expression reads a text or bytes as.
pub fn proven(view: &str) -> String {
    format!("{RUNTIME}::proven({view})")
}

/// `x ?? d`, of the [`view`] of `x` and the spelling of `d`: the present value
/// of `x`, and `d` only where `x` is absent, so `d` is evaluated (and may fail)
/// only then, and `x` once. `d_is_optional` says that `d` is itself an
/// optional value, which makes the whole an optional value: the present `x`
/// is then lifted, as `d` already is one.
pub fn coalesce(view: &str, default: &str, d_is_optional: bool) -> String {
    let taken = if d_is_optional {
        present(PRESENT)
    } else {
        PRESENT.to_string()
    };
    format!("match {view} {{ Some({PRESENT}) => {taken}, None => {default} }}")
}

/// `x === y` (or `x !== y` where `equal` is false) of two optional values, or
/// of an optional value and a [`present`] one, each given as its [`view`]:
/// total, since two absent values are equal and an absent one equals no
/// present one, which is `Option`'s own `==`.
pub fn equality(left: &str, right: &str, equal: bool) -> String {
    let op = if equal { "==" } else { "!=" };
    format!("{left} {op} {right}")
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

    #[test]
    fn the_two_values_are_none_and_some() {
        assert_eq!(expression(ABSENT), "None");
        assert_eq!(expression(&present("7")), "Some(7)");
    }

    #[test]
    fn a_text_or_bytes_is_read_borrowed_and_every_other_element_as_it_is() {
        assert_eq!(expression(&view("self.due", INT64)), "self.due");
        assert_eq!(
            expression(&view("self.status", OptionalElem::Enum)),
            "self.status"
        );
        assert_eq!(
            expression(&view("self.title", OptionalElem::Str)),
            "self.title.as_deref()"
        );
        assert_eq!(
            expression(&view("ev.tag", OptionalElem::Bytes)),
            "ev.tag.as_deref()"
        );
    }

    #[test]
    fn a_borrowed_view_is_owned_again_where_a_place_holds_it() {
        assert_eq!(
            expression(&owned("ev.title.as_deref()", OptionalElem::Str)),
            "ev.title.as_deref().map(str::to_string)"
        );
        assert_eq!(
            expression(&owned("ev.tag.as_deref()", OptionalElem::Bytes)),
            "ev.tag.as_deref().map(<[u8]>::to_vec)"
        );
        assert_eq!(expression(&owned("ev.due", INT64)), "ev.due");
    }

    #[test]
    fn a_present_value_is_mapped_and_an_absent_one_stays_absent() {
        assert_eq!(
            expression(&mapped("ev.title.as_deref().map(str::to_string)", &|v| {
                format!("sce_forge_runtime::algorithm::bounded({v}, 16)?")
            })),
            "match ev.title.as_deref().map(str::to_string) { Some(sce_present) => \
             Some(sce_forge_runtime::algorithm::bounded(sce_present, 16)?), None => None }"
        );
    }

    #[test]
    fn a_test_of_presence_is_is_some_and_of_absence_is_none() {
        assert_eq!(expression(&is_present("self.due")), "self.due.is_some()");
        assert_eq!(
            expression(&is_absent("self.title.as_deref()")),
            "self.title.as_deref().is_none()"
        );
    }

    #[test]
    fn a_proven_read_goes_through_the_runtime() {
        assert_eq!(
            expression(&proven("self.due")),
            "sce_forge_runtime::optional::proven(self.due)"
        );
    }

    #[test]
    fn a_default_is_evaluated_only_in_the_absent_arm() {
        let d = "sce_forge_runtime::algorithm::mul::<i64>(self.huge, self.huge)?";
        assert_eq!(
            expression(&coalesce("self.due", d, false)),
            format!("match self.due {{ Some(sce_present) => sce_present, None => {d} }}")
        );
        assert_eq!(
            expression(&coalesce("self.title.as_deref()", "\"none\"", false)),
            "match self.title.as_deref() { Some(sce_present) => sce_present, None => \"none\" }"
        );
        // An optional default keeps the whole an optional value.
        assert_eq!(
            expression(&coalesce("self.due", "self.other", true)),
            "match self.due { Some(sce_present) => Some(sce_present), None => self.other }"
        );
    }

    #[test]
    fn equality_is_options_own() {
        assert_eq!(
            expression(&equality("self.due", "self.other", true)),
            "self.due == self.other"
        );
        assert_eq!(
            expression(&equality(
                "self.status",
                &present("StaticOptionalStatusEnum::Done"),
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
