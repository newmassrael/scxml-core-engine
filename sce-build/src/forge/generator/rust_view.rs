// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
/// the marker type `walk` ([`list_walk`]), `K` being how many flag inputs an entry's decode
/// takes, held as a list inside the bounds of list field `field` of the codec
/// whose fields are `fields` (`rust_list_bound_args`; docs/adr/0065
/// §6 R2, R7).
///
/// A list of one entry or more that is always present is held as its first
/// entry and the list after it (`NonEmptyListView`), since a view needs a
/// default and a `ListView` of one entry or more has none; a gated one is an
/// `Option` of a `ListView`, whose default is the absent list.
pub(super) fn list_type(
    walk: &str,
    inputs: usize,
    field: &CodecField,
    fields: &[CodecField],
    // The list's law where its entries say an input it hands them
    // (`rust_source::law_type`; docs/adr/0065 §6 R8).
    law: Option<&str>,
) -> String {
    let bounds = super::rust_list_bound_args(field, fields);
    let gated = field.present_if.is_some();
    let list = if !gated && crate::forge::codec_totality::list_least(field) > 0 {
        format!("Bounded<{RUNTIME_CODEC}::NonEmptyListView<'a, {walk}, {inputs}>, {bounds}>")
    } else {
        format!("Bounded<ListView<'a, {walk}, {inputs}>, {bounds}>")
    };
    let list = match law {
        Some(law) => format!("{RUNTIME_CODEC}::Lawful<{list}, {law}>"),
        None => list,
    };
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

/// The name of the walk a codec's module writes for the lists whose entries are that codec
/// ([`entry_walk_item`]): `{Codec}Walk`, `type_name` being the codec's type name.
pub(super) fn entry_walk_name(type_name: &str) -> String {
    format!("{type_name}Walk")
}

/// How one list of a view reads its entries: the marker type its field names, and the item
/// that stands for it in the parent's module.
pub(super) struct ListWalkOf {
    /// The type the list's field names as its `W` ([`list_type`]).
    pub ty: String,
    /// The definition of the list's own marker type, or the alias that keeps its name for the
    /// walk it shares.
    pub item: String,
}

/// How list field `field` of codec `codec` reads its entries, which are the codec imported as
/// `import`: `entry`, `entry_ty`, `inputs` and `advance` as [`walk_item`] takes them.
///
/// A list that carries nothing from one entry to the next (`advance` empty) and hands its
/// entry every input the entry declares reads an entry exactly as every other such list of the
/// same entry does: the entry's view decoded with the inputs in the order the entry declares
/// them. Such a list names the one walk the entry's module writes ([`entry_walk_item`]), so a
/// program that walks lists of one entry from several codecs holds one walk and not one per
/// list. Its own name, `{Codec}{Field}Walk`, stays as an alias of that walk, so a consumer
/// that named it names the same type. A list that carries a value, or whose entry takes its
/// tag from its caller, has a walk of its own.
pub(super) fn list_walk(
    codec: &str,
    field: &str,
    entry: &str,
    entry_ty: &str,
    import: Option<&ImportContext>,
    inputs: usize,
    advance: &[String],
) -> ListWalkOf {
    let own = walk_name(codec, field);
    let shared = import.filter(|imp| {
        advance.is_empty()
            && !imp.codec_variant_is_caller_tag
            && imp.codec_flag_inputs.len() == inputs
    });
    match shared {
        Some(imp) => {
            let ty = format!(
                "super::{}::{}",
                imp.namespace,
                entry_walk_name(&imp.type_name)
            );
            ListWalkOf {
                item: format!(
                    "/// How `{field}` reads its entries: the walk of every list of `{entry}` that\n\
                     /// carries nothing from one entry to the next.\n\
                     #[allow(dead_code)]\n\
                     pub type {own} = {ty};\n"
                ),
                ty,
            }
        }
        None => ListWalkOf {
            item: walk_item(&own, entry, entry_ty, inputs, advance),
            ty: own,
        },
    }
}

/// The walk a codec's module writes for every list whose entries are that codec and that
/// carries nothing from one entry to the next ([`list_walk`]): `{Codec}Walk`, whose entry is
/// the codec's view, read with the `inputs` flag inputs the codec declares. `struct_name` is
/// the codec's type name and `view_borrowed` whether its view carries a lifetime.
pub(super) fn entry_walk_item(struct_name: &str, view_borrowed: bool, inputs: usize) -> String {
    let entry = format!("{struct_name}View");
    let entry_ty = if view_borrowed {
        format!("{entry}<'b>")
    } else {
        entry.clone()
    };
    format!(
        "/// How a list of `{struct_name}` reads its entries when it carries nothing from one\n\
         /// entry to the next: one walk for every such list of every codec.\n\
         {}",
        walk_item(
            &entry_walk_name(struct_name),
            &entry,
            &entry_ty,
            inputs,
            &[]
        )
    )
}

/// The marker type of one list and how it reads an entry
/// ([`ListWalk`](sce_forge_runtime::view::ListWalk)): the item a view's list names by type, in
/// place of the two function pointers it used to hold.
///
/// Function pointers over the lifetime of the bytes (`fn(&mut SceCursor<'a>, ..)`) made the list
/// invariant in that lifetime, so a view decoded from a long-lived buffer could not be lent for a
/// shorter time. The entry is `Entry<'b>` for the reader's `'b` here, as a generic associated type
/// says and a function pointer cannot. The read is the entry view's total read (docs/adr/0128),
/// inlined into the walk, so a walk tests no refusal field by field.
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
         impl {RUNTIME_CODEC}::ListWalk<{inputs}> for {walk} {{\n    \
             type Entry<'b> = {entry_ty};\n\
         \n    \
             #[inline(always)]\n    \
             fn read<'b>(_r: &mut SceReader<'b>, _a: &[u8; {inputs}]) -> Self::Entry<'b> {{\n        \
                 {entry}::read(_r{call_args})\n    \
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
    /// The start the list is held to its law from (`rust_law_start`;
    /// docs/adr/0065 §6 R8), `None` for a list with no law.
    pub law_start: Option<&'a str>,
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
    ///
    /// The list is held as a `Bounded` (docs/adr/0065 §6 R2), made by its one
    /// constructor. The scan has refused an entry past the bound already, with
    /// the error the eager decode gives, and a chain closed by an entry flag
    /// with no entry as short (R7), so the constructor's refusal is never
    /// reached on a frame read whole. The constructors are one chain, a
    /// `Result` the field's value settles in the frame's reader
    /// ([`Self::value`]).
    fn build(&self) -> String {
        let args = self
            .inputs
            .iter()
            .map(|i| if i.from_chain { "0" } else { i.expr.as_str() })
            .collect::<Vec<_>>()
            .join(", ");
        let list = format!("ListView::new(_rest, _count, [{args}])");
        // An always-present list of one entry or more is its first entry and
        // the list after it (`list_type`); the scan has read that first entry.
        let bounded = if self.field.present_if.is_none()
            && crate::forge::codec_totality::list_least(self.field) > 0
        {
            format!(
                "{RUNTIME_CODEC}::NonEmptyListView::from_scan({list}).ok_or(CodecError::NeedMoreBytes).and_then(|_l| Bounded::new(_l).ok_or(CodecError::TooManyElements))"
            )
        } else {
            format!("Bounded::new({list}).ok_or(CodecError::TooManyElements)")
        };
        // Each entry the scan read was read under the start, so the list follows
        // its law and this refusal is never reached either (docs/adr/0065 §6 R8).
        match self.law_start {
            Some(start) => format!(
                "{bounded}.and_then(|_l| {RUNTIME_CODEC}::Lawful::new({start}, _l).ok_or(CodecError::PresentIfMismatch))"
            ),
            None => bounded,
        }
    }

    /// The field's value, from `build`, as the field holds it, settled in the
    /// frame's reader (docs/adr/0128): on a frame that failed it is the
    /// field's default, and the reader keeps the first refusal.
    ///
    /// A gated list is settled as the `Option` the field holds, whose default
    /// is the absent list: a gated list of one entry or more is a `ListView`
    /// held to that fewest ([`list_type`]), which has no default of its own,
    /// so settling the list itself would ask the build for one.
    fn value(&self) -> String {
        let list = self.build();
        if self.field.present_if.is_some() {
            format!("cursor.settle({list}.map(Some))")
        } else {
            format!("cursor.settle({list})")
        }
    }
}

/// What the scan of any list begins with: the frame from the list's first
/// entry on, which is what the walk reads from, and the entry count.
const SCAN_OPEN: &str = "let _rest = cursor.rest();\n            \
                         let mut _count: u32 = 0;";

/// A `<sce:repeat>` field of a view: decode `count` or until-EOF entries, each
/// once and for the checks alone, and keep the span.
///
/// A count past the list's bound is refused before the first entry is read
/// (docs/adr/0077 §2), and a list read to the end of the frame refuses the
/// entry past its bound before it reads it, as the eager decode does: a frame
/// cut short there is refused for its count, not for its missing bytes. A view
/// stores no entry, so it reads to the bound and no capacity holds it (§7).
pub(super) fn repeat_decode_stmt(list: &ViewList) -> String {
    use crate::generator::Language;
    let field = list.field;
    let max_count = crate::forge::codec_totality::list_bound(field, list.fields);
    let id = list.id();
    let entry = list.entry;
    let decode_arg = list.decode_arg;
    let count_ref = match &field.bit_size {
        BitSize::Repeat { count_ref } => count_ref,
        _ => unreachable!("a view repeat statement is made for a repeat field"),
    };
    let scan = |indent: &str, head: &str| {
        // A list the protocol bounds by nothing has no entry past its bound
        // (docs/adr/0077 §7).
        let past = max_count
            .map(|max_count| {
                format!(
                    "if _count == {max_count} {{\n{indent}        \
                         cursor.fail(CodecError::TooManyElements);\n{indent}    \
                     }}\n{indent}    "
                )
            })
            .unwrap_or_default();
        // The loop is left where the frame has failed, so a count read before
        // the refusal makes no loop of reads that read nothing (docs/adr/0128).
        format!(
            "{head} {{\n{indent}    \
                 if cursor.failed() {{\n{indent}        \
                     break;\n{indent}    \
                 }}\n{indent}    \
                 {past}{entry}::read(cursor{decode_arg});\n{indent}    \
                 _count += 1;\n{indent}\
             }}"
        )
    };
    let value = list.value();
    match &field.present_if {
        None => {
            let (guard, head) = match count_ref {
                CountRef::LengthField(len_field) => {
                    let count = filters::to_snake_case(len_field.clone());
                    (
                        super::repeat_count_guard(
                            field,
                            list.fields,
                            &count,
                            Language::Rust,
                            "            ",
                        ),
                        format!("for _ in 0..{count}"),
                    )
                }
                CountRef::UntilEof => (String::new(), "while cursor.remaining() > 0".to_string()),
            };
            let scan = scan("            ", &head);
            format!(
                "{{\n            \
                     {guard}{SCAN_OPEN}\n            \
                     {scan}\n            \
                     _out.{id} = {value};\n        \
                 }}"
            )
        }
        Some(pred) => {
            let test = present_if_test_literal(list.fields, pred, crate::generator::Language::Rust);
            let (open, count_decl, head) = match count_ref {
                CountRef::LengthField(len_field) => {
                    let gated = sibling_field_gated(list.fields, len_field.as_str());
                    let count = filters::to_snake_case(len_field.clone());
                    let bind = if gated {
                        String::new()
                    } else {
                        format!("let _n = {count};\n            ")
                    };
                    let guard = super::repeat_count_guard(
                        field,
                        list.fields,
                        "_n",
                        Language::Rust,
                        "            ",
                    );
                    (
                        rust_gated_head(&test, gated.then_some(count.as_str()), "_n"),
                        format!("{bind}{guard}"),
                        "for _ in 0.._n".to_string(),
                    )
                }
                CountRef::UntilEof => (
                    format!("if {test}"),
                    String::new(),
                    "while cursor.remaining() > 0".to_string(),
                ),
            };
            let scan = scan("            ", &head);
            format!(
                "{open} {{\n            \
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
    /// The chain's depth, `None` where the protocol states none.
    pub max_depth: Option<u32>,
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
        terminate_on,
    } = chain;
    let entry_flag_acc = match terminate_on {
        TlvTerminateStrategy::ExhaustOrDepth => None,
        TlvTerminateStrategy::EntryFlag { flag_name } => Some(flag_getter_name(flag_name, lang)),
    };
    let overflow_check =
        tlv_chain_guard(lang, entry_flag_acc.is_some(), *max_depth, "            ");
    let indent = "                ";
    // The loop is left where the frame has failed (docs/adr/0128).
    let mut lines = vec!["if cursor.failed() { break; }".to_string()];
    lines.extend(chain_end_of_frame_stop(
        &entry_flag_acc,
        "if cursor.remaining() == 0 { break; }",
    ));
    // The entry is bound to a local when the loop reads something off it: the
    // flag that ends the chain, the identifier a per-entry flag-input is
    // waiting for.
    if entry_flag_acc.is_some() || !entry_binds.is_empty() {
        lines.push(format!("let _entry = {entry}::read(cursor{decode_arg});"));
    } else {
        lines.push(format!("{entry}::read(cursor{decode_arg});"));
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
    let loop_head = chain_loop_head(lang, *max_depth);
    let value = list.value();
    let scan = format!(
        "{SCAN_OPEN}\n            \
         {more_decl}{loop_head}\n{body}\
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
/// destination where it sits, a gated body installed as its default first, at
/// whatever profile the destination holds it (docs/adr/0077 §7).
pub(super) fn into_builder_embed(id: &str, gated: bool) -> String {
    if gated {
        format!(
            "match &self.{id} {{\n            \
                 Some(_b) => _b.into_builder(_out.{id}.insert(Default::default()))?,\n            \
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
///
/// The list is a `Bounded` in the builder, filled where it sits through its
/// `refill`, which checks the count the walk leaves (docs/adr/0065 §6 R7). One
/// held to its law (`lawful`) is filled from the start of the view's
/// (docs/adr/0065 §6 R8).
pub(super) fn into_builder_list(id: &str, gated: bool, lawful: bool) -> String {
    let fill = |indent: &str, walk: &str| fill_list(indent, walk);
    let refill = |source: &str| refill_from(source, lawful);
    if gated {
        let body = fill("                    ", "_l");
        format!(
            "match &self.{id} {{\n            \
                 Some(_l) => _out.{id}.insert(Default::default()).{}|_vec| {{\n                    \
                     {body}\n                    \
                     Ok(())\n                \
                 }})?,\n            \
                 None => _out.{id} = None,\n        \
             }}",
            refill("_l")
        )
    } else {
        let body = fill("            ", &format!("self.{id}"));
        format!(
            "_out.{id}.{}|_vec| {{\n            \
                 {body}\n            \
                 Ok(())\n        \
             }})?;",
            refill(&format!("self.{id}"))
        )
    }
}

/// The call that fills a builder's list from the view's list `source`: `refill(`,
/// or `refill_from(<its start>, ` for a list held to its law (docs/adr/0065 §6 R8).
fn refill_from(source: &str, lawful: bool) -> String {
    if lawful {
        format!("refill_from({source}.start(), ")
    } else {
        "refill(".to_string()
    }
}

/// `_vec`, which a `refill` hands over cleared, filled from the walk `walk`,
/// each entry pushed as its default and filled where it sits. The list is the
/// destination profile's, so it is reached through the list traits and not as
/// any one container (docs/adr/0077 §7).
fn fill_list(indent: &str, walk: &str) -> String {
    format!(
        "for _e in {walk}.iter() {{\n{indent}    \
             {RUNTIME_CODEC}::SceList::try_push(_vec, Default::default())?;\n{indent}    \
             if let Some(_slot) = {RUNTIME_CODEC}::SceList::last_mut(_vec) {{\n{indent}        \
                 _e.into_builder(_slot)?;\n{indent}    \
             }}\n{indent}\
         }}"
    )
}

/// `into_builder` of an element of a group (docs/adr/0065 §6 R4b): `src` is a
/// reference to the element the view holds and `dest` the place in the builder
/// it fills. A list is filled entry by entry, a body by its own view, and
/// anything else copied.
pub(super) fn into_builder_member(
    field: &CodecField,
    src: &str,
    dest: &str,
    lawful: bool,
) -> String {
    if field.is_repeat() || field.is_tlv_chain() {
        format!(
            "{dest}.{}|_vec| {{\n                    \
                 {}\n                    \
                 Ok(())\n                \
             }})?;",
            refill_from(src, lawful),
            fill_list("                    ", src)
        )
    } else if field.is_embed() {
        format!("{src}.into_builder(&mut {dest})?;")
    } else {
        format!("{dest} = *{src};")
    }
}
