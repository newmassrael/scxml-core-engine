// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The source interface of a CBOR codec (docs/adr/0058, SCE_FORGE.md §4.6.1).
//!
//! A CBOR map has no length, no count and no flag the encoding derives from
//! another element in the sense a positional frame has: the number of entries
//! in the map head is what the encoding counts, and nothing a value says. So
//! every entry is presented, and the interface is one accessor for each: the
//! value itself for a required entry, an `Option` for one that may be absent.
//!
//! The encoding rule is the codec's own, written once against the interface:
//! the map head counts the entries present, and the entries follow in
//! ascending key order with the shortest heads (RFC 8949 §4.2.1). A string or
//! byte string with a bound is presented within it (`Text`, `Bytes`;
//! docs/adr/0077 §3), and a list of records is lent as the language's list of
//! its bound (docs/adr/0086, section 7.4), so only an enum whose carrier is
//! signed and a list of fewer records than its lower count can be refused,
//! and a refusal found by writing into a
//! sink that only counts leaves the caller's sink untouched.

use super::super::*;
use crate::forge::cbor_codec;
use crate::forge::codec_totality::RefusalKind;
use crate::forge::encode_plan::{Carrier, CborItem, CborPlan, CborValue};
use crate::forge::model::CborEntry;

/// What the interface of one codec is made from.
pub(in super::super) struct Input<'a> {
    pub(in super::super) m: &'a CodecModel,
    pub(in super::super) imports: &'a [ImportContext],
    pub(in super::super) l: &'a LangCtx,
    /// `to_pascal_case(m.name)`.
    pub(in super::super) struct_name: &'a str,
    /// `<'a>` when the map borrows a string or bytes entry, otherwise nothing.
    pub(in super::super) struct_lifetime: &'a str,
    /// Whether the function that writes is always inlined (`--inline-encode`).
    pub(in super::super) inline_encode: bool,
}

/// The type an accessor returns for the value of `e` when it is present: a
/// string or byte string within its bounds where it has them (docs/adr/0077
/// §3), so a source cannot present a value the encode would have to refuse. A
/// list of records is lent: a reference to the list the value holds, whose
/// type holds no more records than its bound.
fn value_type(input: &Input<'_>, e: &CborEntry) -> String {
    let value = cbor_codec::rust_view_value_type(input.l, input.m, e, "'_");
    if e.record_list().is_some() {
        format!("&{value}")
    } else {
        value
    }
}

fn accessor_type(input: &Input<'_>, item: &CborItem<'_>) -> String {
    let value = value_type(input, item.entry);
    if item.required {
        value
    } else {
        input.l.codec_optional_of(&value)
    }
}

/// The expression that reads the entry `acc` of `item` out of a value that
/// holds it as the accessor's type does but by value, as the type
/// [`accessor_type`] gives: a list of records lent by reference, and any other
/// entry read as it is held. The read the decoded map's own interface makes.
fn held_read(item: &CborItem<'_>, acc: &str) -> String {
    match (item.value, item.required) {
        (CborValue::RecordList { .. }, true) => format!("&self.{acc}"),
        (CborValue::RecordList { .. }, false) => format!("self.{acc}.as_ref()"),
        _ => format!("self.{acc}"),
    }
}

/// The expression that reads the entry `acc` of `item` out of the owned mirror as
/// the type [`accessor_type`] gives: the `Bounded` a string or a byte string is held
/// in lends it with its bounds, a list of records is lent as the map lends it, and
/// any other entry is read as it is held. The read the mirror's `as_borrowed`
/// reaches through the interface.
fn owned_read(item: &CborItem<'_>, acc: &str) -> String {
    let runtime = cbor_codec::RUST_RUNTIME_CODEC;
    let read = match item.value {
        CborValue::Text => Some(format!("{runtime}::Bounded::as_text")),
        CborValue::Bytes { .. } => Some(format!("{runtime}::Bounded::as_bytes")),
        _ => None,
    };
    match (read, item.required) {
        (Some(read), true) => format!("{read}(&self.{acc})"),
        (Some(read), false) => format!("self.{acc}.as_ref().map({read})"),
        (None, _) => held_read(item, acc),
    }
}

/// The statement that writes the value `v` of `item`, after its key. A refusal
/// is written where the plan's totality names one, and only there.
fn write_value(input: &Input<'_>, plan: &CborPlan<'_>, item: &CborItem<'_>) -> String {
    // A string or byte string is held within its bounds by its type
    // (docs/adr/0077 §3, `{ usize::MAX }` where it has none, §7), so there is
    // nothing to check; the value written is the `str` or slice it holds.
    let held = "v.into_inner()";
    match item.value {
        CborValue::Bool => "cbor::write_bool(w, v)?;".to_string(),
        CborValue::Text => format!("cbor::write_text(w, {held})?;"),
        CborValue::Bytes { .. } => format!("cbor::write_bytes(w, {held})?;"),
        CborValue::Enum { carrier } => {
            let present = item
                .entry
                .leaf_type()
                .expect("an enum entry is a leaf entry");
            let underlying = input.l.codec_carrier_expr(present, "v");
            match carrier {
                // A signed carrier can hold a value no unsigned integer carries.
                Carrier::Signed => format!(
                    "cbor::write_uint(w, u64::try_from({underlying}).map_err(|_| {})?)?;",
                    plan.totality
                        .require(RefusalKind::CborOutOfRange, &item.entry.id)
                ),
                Carrier::Wide => format!("cbor::write_uint(w, {underlying})?;"),
                Carrier::Narrow => format!("cbor::write_uint(w, u64::from({underlying}))?;"),
            }
        }
        CborValue::Uint { wide: true } => "cbor::write_uint(w, v)?;".to_string(),
        CborValue::Uint { wide: false } => "cbor::write_uint(w, u64::from(v))?;".to_string(),
        // A definite-length array of the records, each the map its own struct
        // writes (docs/adr/0086, section 9.2). The list holds no more than its
        // bound by its type; a lower count is refused where the plan names it.
        CborValue::RecordList { min } => {
            // The length is bound first and compared bound, so no comparison of
            // a `len()` call against a literal is written.
            let mut write = "let len = v.as_slice().len();\n".to_string();
            if min > 0 {
                write.push_str(&format!(
                    "if len < {min} {{\n    return Err({});\n}}\n",
                    plan.totality
                        .require(RefusalKind::CborOutOfRange, &item.entry.id)
                ));
            }
            write.push_str(
                "cbor::write_array_head(w, ::sce_forge_runtime::codec::widen_usize(len))?;\n\
                 for record in v.as_slice() {\n    record.encode_cbor(w)?;\n}",
            );
            write
        }
    }
}

/// The source interface of a CBOR codec, as the text its file holds.
pub(in super::super) fn render(input: &Input<'_>) -> String {
    let m = input.m;
    let name = input.struct_name;
    let lt = input.struct_lifetime;
    let trait_name = format!("{name}Source");
    let accessor = |e: &CborEntry| input.l.codec_field_id(&e.id);

    let plan = CborPlan::of(m, input.imports);
    let mut trait_items = String::new();
    let mut ref_items = String::new();
    let mut builder_items = String::new();
    for item in &plan.entries {
        let e = item.entry;
        let (acc, ty) = (accessor(e), accessor_type(input, item));
        trait_items.push_str(&format!(
            "    /// The entry with key {key}{required}.\n    fn {acc}(&self) -> {ty};\n",
            key = e.key,
            required = if item.required { ", required" } else { "" },
        ));
        ref_items.push_str(&format!(
            "    fn {acc}(&self) -> {ty} {{\n        (**self).{acc}()\n    }}\n"
        ));
        builder_items.push_str(&format!(
            "    fn {acc}(&self) -> {ty} {{\n        {}\n    }}\n",
            held_read(item, &acc)
        ));
    }

    // The entries as written: the map head counts the present ones, then each
    // follows in ascending key order.
    let may_fail = !plan.totality.is_total();
    let required = plan.required();
    let mut body: Vec<String> = Vec::new();
    let optional: Vec<&CborItem<'_>> = plan.entries.iter().filter(|item| !item.required).collect();
    if optional.is_empty() {
        body.push(format!("let count: u64 = {required};"));
    } else {
        body.push(format!("let mut count: u64 = {required};"));
        for item in &optional {
            body.push(format!(
                "if src.{}().is_some() {{\n    count += 1;\n}}",
                accessor(item.entry)
            ));
        }
    }
    body.push("cbor::write_map_head(w, count)?;".to_string());
    for &index in &plan.by_key {
        let item = &plan.entries[index];
        let e = item.entry;
        let acc = accessor(e);
        let write = write_value(input, &plan, item);
        if item.required {
            body.push(format!("let v = src.{acc}();"));
            body.push(format!("cbor::write_uint(w, {})?;", e.key));
            body.push(write);
        } else {
            body.push(format!("if let Some(v) = src.{acc}() {{"));
            body.push(format!("    cbor::write_uint(w, {})?;", e.key));
            body.extend(write.lines().map(|line| format!("    {line}")));
            body.push("}".to_string());
        }
    }
    body.push("Ok(())".to_string());
    let body = super::indent(&body, 8);
    plan.totality.assert_all_written(&m.name);

    // A map whose entries can break a bound is written into a sink that only
    // counts first, so a refusal leaves the caller's sink untouched; one whose
    // entries cannot has nothing to find first.
    let (doc, guard) = if may_fail {
        (
            "\x20   /// declared bound is refused before the first byte is written.\n",
            "\x20       Self::count_source(src)?;\n",
        )
    } else {
        (
            "\x20   /// declared bound is not one a value of it can break: the only error is\n\
             \x20   /// the sink's (docs/adr/0065).\n",
            "",
        )
    };
    let result = super::write_result(may_fail, "K");

    // The owned mirror (`{name}Owned<S>`, generated when every string and byte
    // string of the map states a bound) says what it holds through the same
    // interface, so `encode_source` writes it as it writes the view, and the
    // mirror's `as_borrowed` reads each entry from here.
    let owned_impl = if cbor_codec::rust_owned_mirror(m) {
        let mut items = String::new();
        for item in &plan.entries {
            let e = item.entry;
            let (acc, ty) = (accessor(e), accessor_type(input, item));
            let read = owned_read(item, &acc);
            items.push_str(&format!(
                "    fn {acc}(&self) -> {ty} {{\n        {read}\n    }}\n"
            ));
        }
        format!(
            "impl<{param}: {name}Storage> {trait_name} for {name}Owned<{param}> {{\n{items}}}\n\n",
            param = cbor_codec::RUST_STORAGE_PARAM,
        )
    } else {
        String::new()
    };

    format!(
        "// ── Source (ADR 0058) ────────────────────────────────────────────────\n\
         /// What a value of `{name}` says: its entries. The count the map head holds\n\
         /// is not among them; `{name}` counts the entries present\n\
         /// ([`{name}::encode_source`]).\n\
         pub trait {trait_name} {{\n{trait_items}}}\n\n\
         impl<T: {trait_name} + ?Sized> {trait_name} for &T {{\n{ref_items}}}\n\n\
         impl{lt} {trait_name} for {name}{lt} {{\n{builder_items}}}\n\n\
         {owned_impl}\
         #[allow(dead_code)]\n\
         impl{lt} {name}{lt} {{\n\
         \x20   /// Encode what `src` says as one CBOR map into `w`: the entries present in\n\
         \x20   /// ascending key order, every head in its shortest form. An entry past its\n\
         {doc}\
         \x20   pub fn encode_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K) -> {result} {{\n\
         {guard}\
         \x20       Self::write_source(src, w)\n\
         \x20   }}\n\n\
         \x20   /// [`Self::encode_source`] without the pre-pass a parent has already made.\n\
         \x20   #[doc(hidden)]\n\
         {inline}\
         \x20   pub fn write_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K) -> {result} {{\n\
         {body}\
         \x20   }}\n\n\
         {count}\
         }}\n\n\
         {measure}",
        count = super::count_function(&trait_name, "", "", may_fail),
        inline = super::write_attribute(input.inline_encode),
        measure = super::measure_impl(name, lt, &trait_name, input.inline_encode, may_fail),
    )
}
