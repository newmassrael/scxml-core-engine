// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust codec view (ADR 0049): what `{Codec}View` is made of and the
//! statements that decode one.
//!
//! A decoded `{Codec}<'a>` holds each bounded list as a `heapless::Vec<E, N>`,
//! so the record is as large as its lists can be and every move of it copies
//! that. A view holds a list as a [`ListView`](sce_forge_runtime::view::ListView):
//! the frame from the list's first entry on, how many entries there are, and
//! what a walk needs to read them again. The decode of a view is one scan that
//! decodes every entry once, as the eager decode does, and keeps none of them;
//! so it refuses exactly the frames the eager decode refuses, with the same
//! error, and a walk afterwards cannot fail.
//!
//! The scalars, strings, embedded bodies and variants of a view are decoded by
//! the same statements the eager decode uses, with the view type of the body
//! in place of its eager type. Only the lists differ, and they are here.

use super::*;

/// The Rust spelling of the view of the codec imported as `alias`, where it
/// names a type: `{Type}View`, with the `<'a>` its view carries.
pub(super) fn view_type(imports: &[ImportContext], alias: &str, type_name: &str) -> String {
    format!(
        "{type_name}View{}",
        view_lifetime(import_codec_view_borrowed(imports, alias))
    )
}

/// `<'a>` for a view that carries the lifetime, otherwise nothing.
pub(super) fn view_lifetime(borrowed: bool) -> &'static str {
    if borrowed {
        "<'a>"
    } else {
        ""
    }
}

/// The type of a list field of a view: a [`ListView`](sce_forge_runtime::view::ListView) read by
/// the marker type `walk` ([`walk_item`]), `K` being how many flag inputs an entry's decode
/// takes.
pub(super) fn list_type(walk: &str, inputs: usize, gated: bool) -> String {
    let list = format!("ListView<'a, {walk}, {inputs}>");
    if gated {
        format!("Option<{list}>")
    } else {
        list
    }
}

/// The name of the marker type that reads the entries of list field `field` of codec `codec`:
/// `{Codec}{Field}Walk`, both in Pascal case.
pub(super) fn walk_name(codec: &str, field: &str) -> String {
    format!(
        "{}{}Walk",
        filters::to_pascal_case(codec.to_string()),
        filters::to_pascal_case(field.to_string())
    )
}

/// The marker type of one list and how it reads an entry
/// ([`ListWalk`](sce_forge_runtime::view::ListWalk)): the item a view's list names by type, in
/// place of the two function pointers it used to hold.
///
/// Function pointers over the lifetime of the bytes (`fn(&mut SceCursor<'a>, ..)`) made the list
/// invariant in that lifetime, so a view decoded from a long-lived buffer could not be lent for a
/// shorter time. The entry is `Entry<'b>` for the cursor's `'b` here, as a generic associated type
/// says and a function pointer cannot.
///
/// `entry` is the entry's view as a path for a call (`{Entry}View`), `entry_ty` as a type with the
/// lifetime `'b` where it carries one, and `inputs` how many flag inputs its decode takes, each
/// handed over from `args` by position. `advance` rewrites the carried values from `_e`, the entry
/// just read, into `_s`.
pub(super) fn walk_item(
    walk: &str,
    entry: &str,
    entry_ty: &str,
    inputs: usize,
    advance: &[String],
) -> String {
    let call_args: String = (0..inputs).map(|i| format!(", _a[{i}]")).collect();
    let advance = advance.join(" ");
    format!(
        "#[allow(dead_code)]\n\
         pub struct {walk};\n\
         \n\
         impl ListWalk<{inputs}> for {walk} {{\n    \
             type Entry<'b> = {entry_ty};\n\
         \n    \
             fn decode<'b>(_c: &mut SceCursor<'b>, _a: &[u8; {inputs}]) -> Result<Self::Entry<'b>, CodecError> {{\n        \
                 {entry}::decode(_c{call_args})\n    \
             }}\n\
         \n    \
             fn advance(_e: &Self::Entry<'_>, _s: &mut [u8; {inputs}]) {{ {advance} }}\n\
         }}\n"
    )
}

/// One list of a view, as the scan that reads it needs it.
pub(super) struct ViewList<'a> {
    pub field: &'a CodecField,
    pub fields: &'a [CodecField],
    /// The entry's view, as a path for a call (`{Entry}View`).
    pub entry: &'a str,
    /// What each entry's `decode` is handed after the cursor, one by one and
    /// joined (see [`EmbedThreadArgs`]).
    pub inputs: &'a [ThreadInput],
    pub decode_arg: &'a str,
}

impl ViewList<'_> {
    fn id(&self) -> String {
        codec_field_decode_local(&self.field.id, crate::generator::Language::Rust)
    }

    /// The statement that makes the field's value from what the scan counted.
    ///
    /// `args` are what the walk starts with: what the parent fixes for every
    /// entry, evaluated here, after the scan, where the parent's locals are
    /// still in scope; and 0 for what a chain carries from one entry to the
    /// next, which is 0 before the first entry (`ChainEntryBinds::decls`). How an
    /// entry is read is the field's marker type ([`walk_item`]), which the
    /// list's type names.
    fn build(&self) -> String {
        let args = self
            .inputs
            .iter()
            .map(|i| if i.from_chain { "0" } else { i.expr.as_str() })
            .collect::<Vec<_>>()
            .join(", ");
        format!("ListView::new(_rest, _count, [{args}])")
    }

    /// The field's value, from `build`, as the field holds it.
    fn value(&self) -> String {
        let list = self.build();
        if self.field.present_if.is_some() {
            format!("Some({list})")
        } else {
            list
        }
    }
}

/// What the scan of any list begins with: the frame from the list's first
/// entry on, which is what the walk reads from, and the entry count.
const SCAN_OPEN: &str = "let _rest = cursor.peek_slice(cursor.remaining())?;\n            \
                         let mut _count: u32 = 0;";

/// A `<sce:repeat>` field of a view: decode `count` or until-EOF entries, each
/// once and for the checks alone, and keep the span.
///
/// An entry past `max_count` is refused after it decoded, as the eager decode
/// refuses it when the push that follows its decode finds the list full.
pub(super) fn repeat_decode_stmt(list: &ViewList, max_count: u32) -> String {
    let field = list.field;
    let id = list.id();
    let entry = list.entry;
    let decode_arg = list.decode_arg;
    let count_ref = match &field.bit_size {
        BitSize::Repeat { count_ref } => count_ref,
        _ => unreachable!("a view repeat statement is made for a repeat field"),
    };
    let scan = |indent: &str, head: &str| {
        format!(
            "{head} {{\n{indent}    \
                 {entry}::decode(cursor{decode_arg})?;\n{indent}    \
                 if _count == {max_count} {{\n{indent}        \
                     return Err(CodecError::TooManyElements);\n{indent}    \
                 }}\n{indent}    \
                 _count += 1;\n{indent}\
             }}"
        )
    };
    let value = list.value();
    match &field.present_if {
        None => {
            let head = match count_ref {
                CountRef::LengthField(len_field) => {
                    format!("for _ in 0..{}", filters::to_snake_case(len_field.clone()))
                }
                CountRef::UntilEof => "while cursor.remaining() > 0".to_string(),
            };
            let scan = scan("            ", &head);
            format!(
                "{{\n            \
                     {SCAN_OPEN}\n            \
                     {scan}\n            \
                     _out.{id} = {value};\n        \
                 }}"
            )
        }
        Some(pred) => {
            let test = present_if_test_literal(list.fields, pred, crate::generator::Language::Rust);
            let (count_decl, head) = match count_ref {
                CountRef::LengthField(len_field) => {
                    let gated = sibling_field_gated(list.fields, len_field.as_str());
                    (
                        format!(
                            "let _n = {};\n            ",
                            rust_repeat_count_read(gated, len_field)
                        ),
                        "for _ in 0.._n".to_string(),
                    )
                }
                CountRef::UntilEof => (String::new(), "while cursor.remaining() > 0".to_string()),
            };
            let scan = scan("            ", &head);
            format!(
                "if {test} {{\n            \
                     {count_decl}{SCAN_OPEN}\n            \
                     {scan}\n            \
                     _out.{id} = {value};\n        \
                 }} else {{\n            \
                     _out.{id} = None;\n        \
                 }}"
            )
        }
    }
}

/// What a `<sce:tlv-chain>` view scan needs beyond the list itself.
pub(super) struct ViewChain<'a> {
    /// The flag-inputs the chain supplies entry by entry.
    pub entry_binds: &'a ChainEntryBinds,
    pub max_depth: u32,
    pub on_overflow: crate::forge::model::TlvOverflowPolicy,
    pub terminate_on: &'a crate::forge::model::TlvTerminateStrategy,
}

/// A `<sce:tlv-chain>` field of a view: the loop of the eager decode, with the
/// push that keeps the entry replaced by a count. Every check the eager loop
/// makes is made here, in the same order: the entry's own decode, the flag
/// that ends the chain, and the guard after the loop that tells a chain cut
/// short from one that ran past `max-depth` (`tlv_chain_guard`).
///
/// The values an `entry-flag-bind` carries are kept in the locals the loop
/// declares, as in the eager decode; the walk keeps them in the list's
/// arguments, rewritten by the closure `advance`, which makes the same test.
pub(super) fn tlv_chain_decode_stmt(list: &ViewList, chain: &ViewChain) -> String {
    use crate::forge::model::TlvTerminateStrategy;
    use crate::generator::Language;
    let lang = Language::Rust;
    let field = list.field;
    let id = list.id();
    let entry = list.entry;
    let decode_arg = list.decode_arg;
    let ViewChain {
        entry_binds,
        max_depth,
        on_overflow,
        terminate_on,
    } = chain;
    let entry_flag_acc = match terminate_on {
        TlvTerminateStrategy::ExhaustOrDepth => None,
        TlvTerminateStrategy::EntryFlag { flag_name } => Some(flag_getter_name(flag_name, lang)),
    };
    let overflow_check =
        tlv_chain_guard(lang, *on_overflow, entry_flag_acc.is_some(), "            ");
    let indent = "                ";
    let mut lines = vec!["if cursor.remaining() == 0 { break; }".to_string()];
    // The entry is bound to a local when the loop reads something off it: the
    // flag that ends the chain, the identifier a per-entry flag-input is
    // waiting for.
    if entry_flag_acc.is_some() || !entry_binds.is_empty() {
        lines.push(format!(
            "let _entry = {entry}::decode(cursor{decode_arg})?;"
        ));
    } else {
        lines.push(format!("{entry}::decode(cursor{decode_arg})?;"));
    }
    if let Some(acc) = &entry_flag_acc {
        lines.push(format!("_more = _entry.{acc}();"));
    }
    lines.extend(entry_binds.update_lines(field, "_entry", lang, indent));
    lines.push("_count += 1;".to_string());
    if entry_flag_acc.is_some() {
        lines.push("if !_more { break; }".to_string());
    }
    let body = chain_loop_body(indent, Some("            "), &lines);
    let more_decl = more_decl_line(lang, &entry_flag_acc, entry_binds, "            ");
    let value = list.value();
    let scan = format!(
        "{SCAN_OPEN}\n            \
         {more_decl}for _ in 0..{max_depth}u32 {{\n{body}\
         }}{overflow_check}\n            \
         _out.{id} = {value};"
    );
    match &field.present_if {
        None => format!("{{\n            {scan}\n        }}"),
        Some(pred) => {
            let test = present_if_test_literal(list.fields, pred, lang);
            format!(
                "if {test} {{\n            {scan}\n        }} else {{\n            \
                     _out.{id} = None;\n        \
                 }}"
            )
        }
    }
}

/// What a walk of a `<sce:tlv-chain>` carries from one entry to the next, as the statements
/// of [`ListWalk::advance`](sce_forge_runtime::view::ListWalk::advance): each
/// `entry-flag-bind` rewrites its input from the entry just read (`_e`) with the same test the
/// scan's loop makes. Empty for a chain that carries nothing.
pub(super) fn chain_advance(list: &ViewList, entry_binds: &ChainEntryBinds) -> Vec<String> {
    entry_binds
        .binds
        .iter()
        .map(|bind| {
            let position = list
                .inputs
                .iter()
                .position(|i| i.from_chain && i.input == bind.input)
                .expect("an entry-flag-bind names an input the entry codec declares");
            format!(
                "_s[{position}] = u8::from({});",
                entry_binds.previous_entry_test(
                    bind,
                    list.field,
                    "_e",
                    crate::generator::Language::Rust
                )
            )
        })
        .collect()
}

/// `into_builder` of a field a view holds as the codec holds it (a number, a
/// string, bytes, an enum, an optional of those): copied across.
pub(super) fn into_builder_plain(id: &str) -> String {
    format!("_out.{id} = self.{id};")
}

/// `into_builder` of an embedded body: the body's view fills the body in the
/// destination where it sits. `body` is the body's codec type, which a gated
/// body is installed as before it is filled.
pub(super) fn into_builder_embed(id: &str, body: &str, gated: bool) -> String {
    if gated {
        format!(
            "match &self.{id} {{\n            \
                 Some(_b) => _b.into_builder(_out.{id}.insert({body}::default()))?,\n            \
                 None => _out.{id} = None,\n        \
             }}"
        )
    } else {
        format!("self.{id}.into_builder(&mut _out.{id})?;")
    }
}

/// `into_builder` of a list: the destination list is cleared and each entry the
/// walk yields is pushed as the codec's default and filled in the list, so no
/// entry is moved into it whole. The scan bounded the list by the same `N` the
/// destination holds, so the push cannot fail; the arm is there because `push`
/// returns a `Result`.
pub(super) fn into_builder_list(id: &str, entry: &str, gated: bool) -> String {
    let fill = |list: &str, indent: &str, walk: &str| {
        format!(
            "{list}.clear();\n{indent}\
             for _e in {walk}.iter() {{\n{indent}    \
                 {list}.push({entry}::default()).map_err(|_| CodecError::TooManyElements)?;\n{indent}    \
                 if let Some(_slot) = {list}.last_mut() {{\n{indent}        \
                     _e.into_builder(_slot)?;\n{indent}    \
                 }}\n{indent}\
             }}"
        )
    };
    if gated {
        let body = fill("_vec", "                ", "_l");
        format!(
            "match &self.{id} {{\n            \
                 Some(_l) => {{\n                \
                     let _vec = _out.{id}.insert(HeaplessVec::new());\n                \
                     {body}\n            \
                 }}\n            \
                 None => _out.{id} = None,\n        \
             }}"
        )
    } else {
        fill(&format!("_out.{id}"), "        ", &format!("self.{id}"))
    }
}
