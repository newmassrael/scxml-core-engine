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
//! docs/adr/0077 §3), so only an enum whose carrier is signed can be refused,
//! and a refusal found by writing into a
//! sink that only counts leaves the caller's sink untouched.

use super::super::*;
use crate::forge::cbor_codec;
use crate::forge::codec_totality::{self, RefusalKind, Totality};
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
/// §3), so a source cannot present a value the encode would have to refuse.
fn value_type(input: &Input<'_>, e: &CborEntry) -> String {
    cbor_codec::rust_view_value_type(input.l, e, "'_")
}

fn accessor_type(input: &Input<'_>, e: &CborEntry) -> String {
    let value = value_type(input, e);
    if e.is_required() {
        value
    } else {
        input.l.codec_optional_of(&value)
    }
}

/// The expression that reads the entry `acc` of `e` out of the owned mirror as the
/// type [`accessor_type`] gives: the `Bounded` a string or a byte string is held in
/// lends it with its bounds, and any other entry is read as it is held. The read
/// the mirror's `as_borrowed` reaches through the interface.
fn owned_read(_input: &Input<'_>, e: &CborEntry, acc: &str) -> String {
    let runtime = cbor_codec::RUST_RUNTIME_CODEC;
    let read = match e.present_type() {
        SceType::String => Some(format!("{runtime}::Bounded::as_text")),
        SceType::Bytes => Some(format!("{runtime}::Bounded::as_bytes")),
        _ => None,
    };
    match (read, e.is_required()) {
        (Some(read), true) => format!("{read}(&self.{acc})"),
        (Some(read), false) => format!("self.{acc}.as_ref().map({read})"),
        (None, _) => format!("self.{acc}"),
    }
}

/// The statements that write the value `v` of `e`, after its key. A refusal is
/// written where `totality` names one, and only there.
fn write_value(input: &Input<'_>, totality: &Totality, e: &CborEntry) -> Vec<String> {
    let mut out = Vec::new();
    // A string or byte string with bounds is held within them by its type
    // (docs/adr/0077 §3), so there is nothing to check; the value written is
    // the `str` or slice it holds.
    let held = if cbor_codec::rust_bound_args(e).is_some() {
        "v.into_inner()"
    } else {
        "v"
    };
    match cbor_codec::kind(e) {
        "bool" => out.push("cbor::write_bool(w, v)?;".to_string()),
        "text" => out.push(format!("cbor::write_text(w, {held})?;")),
        "bytes" | "bytes_exact" => out.push(format!("cbor::write_bytes(w, {held})?;")),
        "enum" => {
            let SceType::Enum(r) = e.present_type() else {
                unreachable!("`kind` is enum for an enum type");
            };
            let carrier = cbor_codec::enum_carrier(input.imports, &r.alias)
                .expect("the import resolved the enum's carrier");
            let underlying = input.l.codec_carrier_expr(e.present_type(), "v");
            // A signed carrier can hold a value no unsigned integer carries.
            out.push(
                if matches!(
                    carrier,
                    SceType::Int8 | SceType::Int16 | SceType::Int32 | SceType::Int64
                ) {
                    format!(
                        "cbor::write_uint(w, u64::try_from({underlying}).map_err(|_| {})?)?;",
                        totality.require(RefusalKind::CborOutOfRange, &e.id)
                    )
                } else if carrier == SceType::Uint64 {
                    format!("cbor::write_uint(w, {underlying})?;")
                } else {
                    format!("cbor::write_uint(w, u64::from({underlying}))?;")
                },
            );
        }
        _ => {
            if *e.present_type() == SceType::Uint64 {
                out.push("cbor::write_uint(w, v)?;".to_string());
            } else {
                out.push("cbor::write_uint(w, u64::from(v))?;".to_string());
            }
        }
    }
    out
}

/// The source interface of a CBOR codec, as the text its file holds.
pub(in super::super) fn render(input: &Input<'_>) -> String {
    let m = input.m;
    let name = input.struct_name;
    let lt = input.struct_lifetime;
    let trait_name = format!("{name}Source");
    let accessor = |e: &CborEntry| input.l.codec_field_id(&e.id);

    let mut trait_items = String::new();
    let mut ref_items = String::new();
    let mut builder_items = String::new();
    for e in &m.cbor_entries {
        let (acc, ty) = (accessor(e), accessor_type(input, e));
        trait_items.push_str(&format!(
            "    /// The entry with key {key}{required}.\n    fn {acc}(&self) -> {ty};\n",
            key = e.key,
            required = if e.is_required() { ", required" } else { "" },
        ));
        ref_items.push_str(&format!(
            "    fn {acc}(&self) -> {ty} {{\n        (**self).{acc}()\n    }}\n"
        ));
        builder_items.push_str(&format!(
            "    fn {acc}(&self) -> {ty} {{\n        self.{acc}\n    }}\n"
        ));
    }

    // The entries as written: the map head counts the present ones, then each
    // follows in ascending key order.
    let totality = codec_totality::analyse(m, &mut codec_totality::Resolved(input.imports));
    let may_fail = !totality.is_total();
    let required = m.cbor_entries.iter().filter(|e| e.is_required()).count();
    let mut body: Vec<String> = Vec::new();
    let optional: Vec<&CborEntry> = m.cbor_entries.iter().filter(|e| !e.is_required()).collect();
    if optional.is_empty() {
        body.push(format!("let count: u64 = {required};"));
    } else {
        body.push(format!("let mut count: u64 = {required};"));
        for e in &optional {
            body.push(format!(
                "if src.{}().is_some() {{\n    count += 1;\n}}",
                accessor(e)
            ));
        }
    }
    body.push("cbor::write_map_head(w, count)?;".to_string());
    let mut by_key: Vec<&CborEntry> = m.cbor_entries.iter().collect();
    by_key.sort_by_key(|e| e.key);
    for e in by_key {
        let acc = accessor(e);
        let write = write_value(input, &totality, e);
        if e.is_required() {
            body.push(format!("let v = src.{acc}();"));
            body.push(format!("cbor::write_uint(w, {})?;", e.key));
            body.extend(write);
        } else {
            body.push(format!("if let Some(v) = src.{acc}() {{"));
            body.push(format!("    cbor::write_uint(w, {})?;", e.key));
            body.extend(write.into_iter().map(|line| format!("    {line}")));
            body.push("}".to_string());
        }
    }
    body.push("Ok(())".to_string());
    let body = super::indent(&body, 8);
    totality.assert_all_written(&m.name);

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
        for e in &m.cbor_entries {
            let (acc, ty) = (accessor(e), accessor_type(input, e));
            let read = owned_read(input, e, &acc);
            items.push_str(&format!(
                "    fn {acc}(&self) -> {ty} {{\n        {read}\n    }}\n"
            ));
        }
        format!(
            "impl<{param}: {runtime}::CodecStorage> {trait_name} for {name}Owned<{param}> {{\n{items}}}\n\n",
            param = cbor_codec::RUST_STORAGE_PARAM,
            runtime = cbor_codec::RUST_RUNTIME_CODEC,
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
