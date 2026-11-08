// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Interpreter's spelling of the walks over a composed value ([`super`])
//! (docs/adr/0086, section 7.1, the Interpreter row), for the ECMAScript
//! document a `sce-static` statechart is lowered to
//! ([`crate::forge::static_js`]).
//!
//! | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record |
//! |---|---|---|---|
//! | array | string / the text of its bytes | `null` or the value | plain object |
//!
//! The Interpreter is no generated backend: it has no `Language` and no row
//! of [`crate::forge::event_schema_check::ComposedLowering::of`], and its row
//! of the one table is the lowering's own,
//! [`crate::forge::event_schema_check::ComposedLowering::ECMASCRIPT`], which
//! its refusals are named by. Its target ([`crate::forge::static_js::JsTarget`])
//! answers this spelling where a backend's static target answers its own.
//!
//! The data model holds no types, so a type here is the language's own
//! spelling of it, which carries no bound: two places whose types differ only
//! in their bounds are one type, and a value that fits is copied as it is. A
//! list and a record are never changed in place (a statement that changes one
//! writes the whole value again), so a copy shares nothing that could change
//! under it, and the data model holds no integer past 2^53 - 1 (a payload's is
//! refused where it is read, the library's `payload`, and a variable's where
//! it is written, its `out`), so a copy has nothing to check either.
//!
//! A reader is the descriptor of the value's type, the one the library's
//! `payload` and `field` read a value of an event's data by
//! (`static_js/sce_static.js`, `readValue`), every bound of the type in it, so
//! a value outside its type is refused at its path before anything of it is
//! kept, which is how the Interpreter holds a host's value to its bounds
//! (section 5.4): it has no typed seam a host writes a value into, and a value
//! reaches it as the JSON of an event.
//!
//! A writer writes the value a `<send>`'s `<param>` carries, which the
//! Interpreter's own `<send>` writes as the event's JSON (section 5.3): a list
//! as an array of its elements, a record as an object of its fields, an absent
//! value as `null`, and each scalar as a scalar param crosses, a 64-bit integer
//! as the text of its digits.

use super::*;
use crate::forge::static_js::RUNTIME_GLOBAL;

/// The Interpreter's spellings of the walks (section 7.1, the Interpreter
/// row). One spelling serves a read and a send alike: with no seam a host
/// writes a typed value into, a writer only ever writes what a `<send>`
/// carries.
///
/// A writer and a copy are function literals of the value they are handed,
/// bound to [`JS_ELEMENT_BINDING`] or [`JS_COPY_BINDING`]: what such a literal
/// spells reads only its binding and the library, so no variable of the
/// document it would shadow is read there, and a binding nested in another of
/// the same name is its own, the holder read where the outer one is bound.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JsComposed;

/// The name a reader binds a part to. A reader is a descriptor, which binds
/// nothing, so the name is never written.
const JS_READ_BINDING: &str = "scePart";

/// The name a writer binds an element or a present value to.
const JS_ELEMENT_BINDING: &str = "sceElement";

/// The name a copy binds an element or a present value to.
const JS_COPY_BINDING: &str = "sceCopied";

/// Why an Interpreter walk meets no union: the one decision of what is
/// lowered refuses a union for the Interpreter before any walk reaches one
/// ([`crate::forge::event_schema_check::ComposedLowering::unions`]).
const JS_HOLDS_NO_UNION: &str =
    "the Interpreter's lowering holds no union yet (docs/adr/0092, section 7): the one \
     decision of what is lowered refuses a union for it before any walk reaches one";

/// `held`, a list, with `each` (a value of its element bound to `binding`)
/// taken for every element, by the array's own `map`; `held` itself where
/// `each` is the element as it is, which leaves the list as it is.
fn each_element(held: &str, binding: &str, each: &str) -> String {
    if each == binding {
        held.to_string()
    } else {
        format!("{held}.map(function ({binding}) {{ return {each}; }})")
    }
}

/// `held`, a value that may be absent, with `present` (a value of its present
/// value bound to `binding`) taken where it is present and `null` kept, by the
/// library's `mapPresent`; `held` itself where `present` is the present value
/// as it is.
fn each_present(held: &str, binding: &str, present: &str) -> String {
    if present == binding {
        held.to_string()
    } else {
        format!("{RUNTIME_GLOBAL}.mapPresent({held}, function ({binding}) {{ return {present}; }})")
    }
}

impl ComposedSpelling for JsComposed {
    // The types are the language's own spellings, as the target names a
    // variable's (`JsTarget::scalar_type`, `JsTarget::list_type`), none with a
    // bound in it.
    fn scalar_type(&self, ty: &SceType) -> String {
        ty.as_attr()
    }
    fn text_type(&self, _max: u32) -> String {
        "string".to_string()
    }
    fn bytes_type(&self, _max: u32) -> String {
        "bytes".to_string()
    }
    fn optional_type(&self, elem: &str) -> String {
        format!("optional<{elem}>")
    }
    fn list_type(&self, elem: &str, _max: u32) -> String {
        format!("list<{elem}>")
    }
    fn record_type(&self, alias: &str) -> String {
        format!("record:{alias}")
    }
    // Nothing is declared: a record is a plain object of its fields.
    fn record_def(
        &self,
        _ty: &str,
        _alias: &str,
        _fields: &[(String, String)],
        _checks: &[String],
    ) -> String {
        String::new()
    }

    // A reader is the descriptor of the type: the library reads the value it is
    // handed by it, so `value` names nothing here.
    fn read_binding(&self) -> &'static str {
        JS_READ_BINDING
    }
    // The type's own name, the key the library holds a number to its width by.
    fn read_scalar(&self, ty: &SceType, _value: &str) -> String {
        format!("'{}'", ty.as_attr())
    }
    // The names the enum declares, as `field` reads an enum field by them.
    fn read_enum(&self, held: &MachineEnum, _value: &str) -> String {
        let names: Vec<&str> = held
            .variants
            .iter()
            .map(|(declared, _)| declared.as_str())
            .collect();
        crate::forge::static_js::variant_list(&names)
    }
    fn read_text(&self, max: u32, _value: &str) -> String {
        format!("{{string: {max}}}")
    }
    fn read_bytes(&self, max: u32, _value: &str) -> String {
        format!("{{bytes: {max}}}")
    }
    fn read_optional(&self, _value: &str, elem: &str, _elem_ty: &str) -> String {
        format!("{{optional: {elem}}}")
    }
    // The bound, and a lower count where the type states one, which the library
    // holds before any element is read (docs/adr/0086, section 15.5).
    fn read_list(&self, _value: &str, least: u32, max: u32, elem: &str, _elem_ty: &str) -> String {
        if least == 0 {
            format!("{{list: {elem}, max: {max}}}")
        } else {
            format!("{{list: {elem}, max: {max}, min: {least}}}")
        }
    }
    // Each field in the schema's order, and the JSON of the fields a later
    // revision added, which is an object literal here, read where a delivery
    // lacks one.
    fn read_record(
        &self,
        _value: &str,
        defaults: Option<&str>,
        _ty: &str,
        fields: &[(String, String)],
    ) -> String {
        let members: Vec<String> = fields
            .iter()
            .map(|(id, read)| format!("'{id}': {read}"))
            .collect();
        let defaults = defaults
            .map(|json| format!(", defaults: {json}"))
            .unwrap_or_default();
        format!("{{record: {{{}}}{defaults}}}", members.join(", "))
    }

    fn write_binding(&self) -> &'static str {
        JS_ELEMENT_BINDING
    }
    // As a scalar param crosses (`JsTarget::wire_value`): a 64-bit integer as
    // the text of its digits, every other number and a truth value as held.
    fn write_scalar(&self, ty: &SceType, held: &Held) -> String {
        if matches!(ty, SceType::Uint64 | SceType::Int64) {
            format!("String({})", held.text)
        } else {
            held.text.clone()
        }
    }
    // An enum value is already the name its enum declares.
    fn write_enum(&self, held: &Held) -> String {
        held.text.clone()
    }
    fn write_text(&self, held: &Held) -> String {
        held.text.clone()
    }
    // A byte string is already the text of its bytes, one character to a byte,
    // which is what the wire carries.
    fn write_bytes(&self, held: &Held) -> String {
        held.text.clone()
    }
    // `null` where the value is absent, kept by `mapPresent`, which hands the
    // present value on; presence is `null`, never a truth test, so a present
    // `0` is written as the value it is (docs/adr/0055, section 4).
    fn write_optional(&self, held: &Held, elem: &str) -> String {
        each_present(&held.text, JS_ELEMENT_BINDING, elem)
    }
    fn write_list(&self, held: &Held, elem: &str) -> String {
        each_element(&held.text, JS_ELEMENT_BINDING, elem)
    }
    fn write_field(&self, held: &Held, field: &str) -> String {
        format!("{}.{field}", held.text)
    }
    // Parenthesised, as `JsTarget::record_value` writes a record: `{a: 1}` alone
    // reads as a block.
    fn write_record(&self, fields: &[(String, String)]) -> String {
        let members: Vec<String> = fields
            .iter()
            .map(|(id, value)| format!("'{id}': {value}"))
            .collect();
        format!("({{{}}})", members.join(", "))
    }

    fn copy_binding(&self) -> &'static str {
        JS_COPY_BINDING
    }
    // Nothing is changed in place and nothing past what the data model holds is
    // held, so the value is its own copy.
    fn copy_whole(&self, held: &Held, _plain: bool) -> String {
        held.text.clone()
    }
    // A text and a byte string are one type whatever their bound, so a walk that
    // copies by type never reaches these; the value is the copy.
    fn copy_text(&self, _max: u32, held: &Held) -> String {
        held.text.clone()
    }
    fn copy_bytes(&self, _max: u32, held: &Held) -> String {
        held.text.clone()
    }
    fn copy_optional(&self, held: &Held, elem: &str, _elem_ty: &str) -> String {
        each_present(&held.text, JS_COPY_BINDING, elem)
    }
    // A value of `T` stands where an `optional<T>` is expected as it is.
    fn copy_present(&self, value: &str) -> String {
        value.to_string()
    }
    fn copy_list(&self, held: &Held, elem: &str, _elem_ty: &str) -> String {
        each_element(&held.text, JS_COPY_BINDING, elem)
    }

    // The Interpreter's lowering holds no union yet: the gate refuses one by
    // name before a walk reaches it, so each spelling below either names what it
    // would be or says why there is none.
    fn union_type(&self, name: &str) -> String {
        format!("union:{name}")
    }
    fn union_case_ident(&self, variant: &str) -> String {
        variant.to_string()
    }
    fn union_tag_read(&self, _tag: &str) -> Option<String> {
        None
    }
    fn union_case_read(&self, variant: &str) -> String {
        format!("proven_{variant}")
    }
    fn union_shared_read(&self, field: &str) -> String {
        format!("proven_field_{field}")
    }
    fn union_def(&self, _def: &UnionDef) -> String {
        unreachable!("{JS_HOLDS_NO_UNION}")
    }
    fn union_value(&self, _ty: &str, _case: &str, _record: Option<&str>) -> String {
        unreachable!("{JS_HOLDS_NO_UNION}")
    }
    fn read_union(&self, _value: &str, _read: &UnionRead<'_>) -> String {
        unreachable!("{JS_HOLDS_NO_UNION}")
    }
    fn write_union(&self, _held: &Held, _ty: &str, _tag: &str, _arms: &[UnionWriteArm]) -> String {
        unreachable!("{JS_HOLDS_NO_UNION}")
    }
    fn union_member(
        &self,
        _union: &ScopedUnion,
        _left: &[&ScopedCase],
        _member: &str,
    ) -> Result<String, String> {
        Err(JS_HOLDS_NO_UNION.to_string())
    }
    fn union_place_of(&self, read: &str) -> String {
        format!("{read}_place")
    }
    fn union_member_place(
        &self,
        _holder: &str,
        _union: &ScopedUnion,
        _left: &[&ScopedCase],
        _member: &str,
    ) -> Result<String, String> {
        Err(JS_HOLDS_NO_UNION.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    /// The Interpreter's row of the one table of what is lowered of a composed
    /// value: a payload read at any depth, a list taken whole and a composed
    /// value sent (docs/adr/0086, sections 5 and 6), and no union held yet
    /// (docs/adr/0092, section 7), named as its refusals are.
    #[test]
    fn the_interpreter_row_lowers_every_composed_position_but_a_union() {
        use crate::forge::event_schema_check::ComposedLowering;
        assert_eq!(
            ComposedLowering::ECMASCRIPT,
            ComposedLowering {
                name: "ecmascript",
                reads: true,
                copies: true,
                unions: false,
                sends: true,
            }
        );
    }

    /// The Interpreter row of section 7.1: the language's own spelling of each
    /// type, no bound in it, at any depth; nothing is declared for a record.
    #[test]
    fn an_interpreter_type_is_the_language_spelling_with_no_bound() {
        let held = held();
        let enums = enums();
        let spelled =
            |text: &str| machine_type(&ty(text), None, &held, &enums, &JsComposed).expect("a type");
        assert_eq!(spelled("uint16"), "uint16");
        assert_eq!(spelled("string[8]"), "string");
        assert_eq!(spelled("bytes[4]"), "bytes");
        assert_eq!(spelled("optional<int64>"), "optional<int64>");
        assert_eq!(spelled("list<list<uint8>[2]>[3]"), "list<list<uint8>>");
        assert_eq!(spelled("list<record:Item>[3]"), "list<record:Item>");
        assert_eq!(spelled("optional<list<uint8>[2]>"), "optional<list<uint8>>");
        assert_eq!(spelled("list<string[4]>[2]"), spelled("list<string[8]>[3]"));
        let members = record_members("Item", &held, &enums, &JsComposed).expect("members");
        assert_eq!(
            JsComposed.record_def("record:Item", "Item", &members, &[]),
            ""
        );
    }

    /// An Interpreter reader is the descriptor the library reads a value by:
    /// a number by its type's name, an enum by the names it declares, a text
    /// and a byte string by their bounds, a list by its bound and its lower
    /// count, a record by its fields in the schema's order with the defaults
    /// of the fields a later revision added.
    #[test]
    fn an_interpreter_reader_is_the_descriptor_of_each_constructor() {
        let held = held();
        let enums = enums();
        let read =
            |text: &str| reader(&ty(text), None, &held, &enums, &JsComposed, "v").expect("read");
        assert_eq!(read("uint8"), "'uint8'");
        assert_eq!(read("int64"), "'int64'");
        assert_eq!(read("float32"), "'float32'");
        assert_eq!(read("bool"), "'bool'");
        assert_eq!(read("string[8]"), "{string: 8}");
        assert_eq!(read("bytes[4]"), "{bytes: 4}");
        assert_eq!(read("enum:Level"), "['low', 'high']");
        assert_eq!(read("optional<uint16>"), "{optional: 'uint16'}");
        assert_eq!(
            read("list<list<uint8>[2]>[3]"),
            "{list: {list: 'uint8', max: 2}, max: 3}"
        );
        assert_eq!(
            read("list<list<uint8>[1..2]>[2..3]"),
            "{list: {list: 'uint8', max: 2, min: 1}, max: 3, min: 2}"
        );
        assert_eq!(
            read("record:Item"),
            "{record: {'id': 'uint32', 'level': ['low', 'high'], 'due': {optional: 'int64'}, \
             'blob': {bytes: 4}, 'tags': {list: {string: 8}, max: 4}}}"
        );
        // A list the second revision added reads empty where a delivery lacks it.
        assert_eq!(
            read("record:Batch"),
            "{record: {'items': {list: {record: {'id': 'uint32', 'level': ['low', 'high'], \
             'due': {optional: 'int64'}, 'blob': {bytes: 4}, 'tags': {list: {string: 8}, max: \
             4}}}, max: 3}, 'grid': {list: {list: 'uint8', max: 2}, max: 2}, 'late': {list: \
             'uint8', max: 2}}, defaults: {\"late\":[]}}"
        );
    }

    /// The value an Interpreter `<send>` carries a composed param as: a list
    /// mapped element by element only where an element changes as it crosses,
    /// a record as an object of its fields, an absent value kept `null`, a
    /// 64-bit integer as the text of its digits, and every other scalar, an
    /// enum value and a byte string as held.
    #[test]
    fn an_interpreter_send_writes_only_what_changes_as_it_crosses() {
        let held = held();
        let write =
            |text: &str, at: Held| writer(&ty(text), None, &held, &JsComposed, &at).expect("write");
        assert_eq!(write("uint8", Held::place("x.n")), "x.n");
        assert_eq!(write("int64", Held::place("x.n")), "String(x.n)");
        assert_eq!(write("uint64", Held::place("x.n")), "String(x.n)");
        assert_eq!(write("enum:Level", Held::place("x.level")), "x.level");
        assert_eq!(write("bytes[4]", Held::place("x.blob")), "x.blob");
        assert_eq!(
            write("list<list<uint8>[2]>[2]", Held::place("x.grid")),
            "x.grid"
        );
        assert_eq!(
            write("optional<list<uint8>[2]>", Held::place("x.spare")),
            "x.spare"
        );
        assert_eq!(
            write("list<optional<int64>>[2]", Held::place("x.maybe")),
            "x.maybe.map(function (sceElement) { return SceStatic.mapPresent(sceElement, \
             function (sceElement) { return String(sceElement); }); })"
        );
        assert_eq!(
            write("list<record:Item>[3]", Held::place("x.items")),
            "x.items.map(function (sceElement) { return ({'id': sceElement.id, 'level': \
             sceElement.level, 'due': SceStatic.mapPresent(sceElement.due, function \
             (sceElement) { return String(sceElement); }), 'blob': sceElement.blob, 'tags': \
             sceElement.tags}); })"
        );
        assert_eq!(JsComposed.send_param("x".to_string()), "x");
    }

    /// Two places whose types differ only in their bounds are one type, so a
    /// value that fits is copied as it is; a list whose element becomes the
    /// present value of an optional one is the same array, since a present
    /// value is the value itself.
    #[test]
    fn an_interpreter_copy_of_a_value_that_fits_is_the_value() {
        let held = held();
        let enums = enums();
        let copy = |from: &str, to: &str| {
            copier(
                &ty(from),
                None,
                &ty(to),
                None,
                &held,
                &enums,
                &JsComposed,
                &Held::place("x"),
            )
        };
        assert_eq!(
            copy("list<uint8>[2]", "list<uint8>[4]"),
            Ok("x".to_string())
        );
        assert_eq!(
            copy("list<string[4]>[2]", "list<string[8]>[3]"),
            Ok("x".to_string())
        );
        assert_eq!(
            copy("list<uint8>[2]", "list<optional<uint8>>[2]"),
            Ok("x".to_string())
        );
        assert_eq!(
            copy(
                "optional<list<uint8>[2]>",
                "optional<list<optional<uint8>>[2]>"
            ),
            Ok("x".to_string())
        );
    }

    /// An Interpreter walk names no union: the one decision of what is lowered
    /// refuses one for the Interpreter first, and the walk that would declare
    /// one says why there is none.
    #[test]
    fn an_interpreter_walk_reads_no_union() {
        let shapes = shapes();
        assert!(union_definition(SHAPE_KEY, &shapes, &kinds(), &JsComposed).is_err());
        let all: Vec<&ScopedCase> = shapes.union.cases.iter().collect();
        assert!(JsComposed
            .union_member(&shapes.union, &all, "kind")
            .unwrap_err()
            .contains("holds no union"));
    }
}
