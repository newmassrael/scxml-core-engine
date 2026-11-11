// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Interpreter's spelling of the walks over a composed value ([`super`])
//! (docs/adr/0086, section 7.1, the Interpreter row), for the ECMAScript
//! document a `sce-static` statechart is lowered to
//! ([`crate::forge::static_js`]).
//!
//! | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record | `union:U` |
//! |---|---|---|---|---|
//! | array | string / the text of its bytes | `null` or the value | plain object | plain object, its tag beside its case's fields |
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
//!
//! A union (docs/adr/0092) is the wire's own object, as the library reads one
//! (`readValue`): its tag's key holds the declared name of the case's variant
//! beside the fields of the case's record, and no other key. So a member is
//! read by its own name wherever the narrowing admits the read, a value is
//! copied and sent as it is, and a field is written by writing the holder
//! whole. The Interpreter keeps no saved state of its own, so nothing is
//! written for one.

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

/// What the writer of a union throws for a value whose tag names none of its
/// cases, which nothing the data model holds of a union is: every one is read,
/// built or lifted as a case.
const JS_UNION_IN_NO_CASE: &str = "a value of a union whose tag names none of its cases";

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
    // The count held first by the library's `holdsAtLeast`, which throws a
    // `precondition` failure as a checked operation does, then the list
    // written: the comma reads them in that order.
    fn send_counted(&self, held: &Held, _field: &str, least: u32, sent: String) -> Option<String> {
        Some(format!(
            "(SceStatic.holdsAtLeast({}, {least}), {sent})",
            held.text
        ))
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

    // A union is the wire's own object (docs/adr/0092, section 2.1, and section
    // 7, the Interpreter row): the tag's key holding the declared name of the
    // case's variant, which is what the data model holds a value of the tag's
    // enum as, beside the fields of the case's record. Its type is named only
    // where a refusal reads it, and a case by the declared name of its variant.
    fn union_type(&self, name: &str) -> String {
        format!("union:{name}")
    }
    fn union_case_ident(&self, variant: &str) -> String {
        variant.to_string()
    }
    // The tag is the member of its own key.
    fn union_tag_read(&self, tag: &str) -> Option<String> {
        Some(tag.to_string())
    }
    // The object holds the fields of its case itself, so no read of a case's
    // record or of a shared field is declared: these names only keep the reads
    // a declaration lists apart, and nothing reads them.
    fn union_case_read(&self, variant: &str) -> String {
        format!("proven_{variant}")
    }
    fn union_shared_read(&self, field: &str) -> String {
        format!("proven_field_{field}")
    }
    // Nothing is declared: the data model holds no types.
    fn union_def(&self, _def: &UnionDef) -> String {
        String::new()
    }
    // The case's record with the tag's key set to the case's declared name, by
    // the library's `set`, as a record's field is written; the tag alone for an
    // empty case. Parenthesised, as `JsTarget::record_value` writes a record.
    fn union_value(&self, _ty: &str, tag: &str, case: &str, record: Option<&str>) -> String {
        let (key, name) = (js_string(tag), js_string(case));
        match record {
            Some(record) => format!("{RUNTIME_GLOBAL}.set({record}, {key}, {name})"),
            None => format!("({{{key}: {name}}})"),
        }
    }
    // The descriptor the library reads a union by
    // (`static_js/sce_static.js`, `readValue`): the tag's key, and each case by
    // the declared name of its variant, holding the descriptor of its record or
    // `null` for an empty case. The tag is read first, as an enum of those
    // names, and the record of the case it names from the same object.
    fn read_union(&self, _value: &str, read: &UnionRead<'_>) -> String {
        let cases: Vec<String> = read
            .arms
            .iter()
            .map(|arm| {
                format!(
                    "{}: {}",
                    js_string(&arm.case_ident),
                    arm.record.as_deref().unwrap_or("null")
                )
            })
            .collect();
        format!(
            "{{union: {{tag: {}, cases: {{{}}}}}}}",
            js_string(read.tag),
            cases.join(", ")
        )
    }
    // The object as it is where no field of any case changes as it crosses:
    // what the data model holds of a union is already the wire's form, the tag
    // beside its case's fields and no other key. Otherwise each case written by
    // its arm, the tag's key and the case's fields of the value bound to
    // [`JS_ELEMENT_BINDING`]; a value whose tag names no case, which nothing the
    // data model holds is, throws rather than crosses as nothing.
    fn write_union(&self, held: &Held, _ty: &str, tag: &str, arms: &[UnionWriteArm]) -> String {
        let as_held = arms.iter().all(|arm| {
            arm.fields
                .iter()
                .flatten()
                .all(|(id, value)| *value == format!("{JS_ELEMENT_BINDING}.{id}"))
        });
        if as_held {
            return held.text.clone();
        }
        let key = js_string(tag);
        let cases: String = arms
            .iter()
            .map(|arm| {
                let name = js_string(&arm.variant);
                let fields: String = arm
                    .fields
                    .iter()
                    .flatten()
                    .map(|(id, value)| format!(", {}: {value}", js_string(id)))
                    .collect();
                format!("case {name}: return ({{{key}: {name}{fields}}}); ")
            })
            .collect();
        format!(
            "(function ({JS_ELEMENT_BINDING}) {{ switch ({JS_ELEMENT_BINDING}[{key}]) {{ {cases}}} \
             throw new Error({}); }})({})",
            js_string(JS_UNION_IN_NO_CASE),
            held.text
        )
    }
    // A member is read by its own name: the tag, or a field every case left
    // declares, which the object holds under that name in each of them.
    fn union_member(
        &self,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        if member == union.tag
            || (!left.is_empty()
                && left
                    .iter()
                    .all(|case| case_field(union, case, member).is_some()))
        {
            return Ok(member.to_string());
        }
        Err(format!(
            "`{member}` of `union:{}`, read where it may be in the cases {}, is a field not every \
             case left declares (docs/adr/0092, section 6.1)",
            union.name,
            cases_named(left)
        ))
    }
    fn union_place_of(&self, read: &str) -> String {
        format!("{read}_place")
    }
    // No field is written in place: a union, as a record, is written whole
    // ([`Self::union_member_write`]).
    fn union_member_place(
        &self,
        _holder: &str,
        union: &ScopedUnion,
        _left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        Err(format!(
            "`{member}` of `union:{}` has no place of its own in the Interpreter's data model: \
             the union is written whole",
            union.name
        ))
    }
    // The holder written again with the field changed, by the library's `set`,
    // as a record's field is (`JsTarget::field_assignment`): a copy of the
    // machine's state taken earlier keeps what it saw. The tag is never written
    // alone (docs/adr/0092, section 6.1 item 6), and a field is written only
    // where every case left declares it.
    fn union_member_write(
        &self,
        holder: &str,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<UnionMemberWrite, String> {
        if member == union.tag {
            return Err(format!(
                "the tag `{member}` of `union:{}` is never written alone",
                union.name
            ));
        }
        let field = js_string(&self.union_member(union, left, member)?);
        let written = holder.to_string();
        let next: UnionNext = Box::new(move |value: &str| {
            format!("{RUNTIME_GLOBAL}.set({written}, {field}, {value})")
        });
        Ok(UnionMemberWrite::Holder {
            holder: holder.to_string(),
            next,
        })
    }
}

/// `text` as a single-quoted ECMAScript string: the one ECMAScript quoter
/// ([`crate::filters::ecmascript_string_literal`]).
fn js_string(text: &str) -> String {
    crate::filters::ecmascript_string_literal(text)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    /// The Interpreter's row of the one table of what is lowered of a composed
    /// value: a payload read at any depth, a list taken whole and a composed
    /// value sent (docs/adr/0086, sections 5 and 6), and a union held
    /// (docs/adr/0092, section 7), named as its refusals are; no variable of a
    /// record that holds a composition yet.
    #[test]
    fn the_interpreter_row_lowers_every_composed_position_but_a_composed_record() {
        use crate::forge::event_schema_check::ComposedLowering;
        assert_eq!(
            ComposedLowering::ECMASCRIPT,
            ComposedLowering {
                name: "ecmascript",
                reads: true,
                copies: true,
                unions: true,
                records: false,
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
        // A list sent where its field asks a lower count it does not prove is
        // held to the count before it is written (docs/adr/0086, section 15.6).
        assert_eq!(
            JsComposed.send_counted(&Held::place("taking"), "ops", 1, "W".to_string()),
            Some("(SceStatic.holdsAtLeast(taking, 1), W)".to_string())
        );
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

    /// An Interpreter union is the wire's own object (docs/adr/0092, section 7,
    /// the Interpreter row): declared as nothing, read by the descriptor of its
    /// tag and of each case by the declared name of its variant, built as the
    /// case's record with the tag's key set, written for a `<send>` as it is
    /// where no field changes as it crosses, and copied as it is.
    #[test]
    fn an_interpreter_union_is_the_wire_object() {
        let shapes = shapes();
        let kinds = kinds();
        let def = union_definition(SHAPE_KEY, &shapes, &kinds, &JsComposed).expect("declared");
        assert_eq!(def.ty, "union:shape");
        assert_eq!(def.tag_method, "kind");
        assert_eq!(JsComposed.union_def(&def), "");
        assert_eq!(
            reader(&ty("union:Shape"), None, &shapes, &kinds, &JsComposed, "v").expect("read"),
            "{union: {tag: 'kind', cases: {'dot': null, 'circle': {record: {'radius': \
             'uint16'}}, 'disc': {record: {'radius': 'uint16'}}}}}"
        );
        assert_eq!(
            JsComposed.union_value("union:shape", "kind", "dot", None),
            "({'kind': 'dot'})"
        );
        assert_eq!(
            JsComposed.union_value("union:shape", "kind", "circle", Some("({'radius': 2})")),
            "SceStatic.set(({'radius': 2}), 'kind', 'circle')"
        );
        assert_eq!(
            writer(
                &ty("union:Shape"),
                None,
                &shapes,
                &JsComposed,
                &Held::place("x.s")
            )
            .expect("write"),
            "x.s"
        );
        assert_eq!(
            copier(
                &ty("union:Shape"),
                None,
                &ty("union:Shape"),
                None,
                &shapes,
                &kinds,
                &JsComposed,
                &Held::place("x.s"),
            ),
            Ok("x.s".to_string())
        );
    }

    /// A union whose case holds a field that changes as it crosses is written
    /// case by case, the tag's key beside the case's fields, and a value in no
    /// case throws rather than crosses as nothing.
    #[test]
    fn an_interpreter_union_is_written_case_by_case_where_a_field_changes() {
        let at = Held::place("x.s");
        let arms = [
            UnionWriteArm {
                case_ident: "dot".to_string(),
                variant: "dot".to_string(),
                fields: None,
            },
            UnionWriteArm {
                case_ident: "stamp".to_string(),
                variant: "stamp".to_string(),
                fields: Some(vec![(
                    "at".to_string(),
                    "String(sceElement.at)".to_string(),
                )]),
            },
        ];
        assert_eq!(
            JsComposed.write_union(&at, "union:u", "kind", &arms),
            "(function (sceElement) { switch (sceElement['kind']) { case 'dot': return \
             ({'kind': 'dot'}); case 'stamp': return ({'kind': 'stamp', 'at': \
             String(sceElement.at)}); } throw new Error('a value of a union whose tag names \
             none of its cases'); })(x.s)"
        );
    }

    /// A member is read by its own name where every case left declares it, and
    /// a field is written by writing the holder whole with the field changed;
    /// the tag is never written alone, and a field another case left does not
    /// declare is neither read nor written.
    #[test]
    fn an_interpreter_union_member_is_read_by_name_and_written_whole() {
        let shapes = shapes();
        let union = &shapes.union;
        let all: Vec<&ScopedCase> = union.cases.iter().collect();
        let round: Vec<&ScopedCase> = union
            .cases
            .iter()
            .filter(|case| case.variant.name != "dot")
            .collect();
        assert_eq!(
            JsComposed.union_member(union, &all, "kind"),
            Ok("kind".to_string())
        );
        assert_eq!(
            JsComposed.union_member(union, &round, "radius"),
            Ok("radius".to_string())
        );
        assert!(JsComposed.union_member(union, &all, "radius").is_err());
        match JsComposed.union_member_write("built", union, &round, "radius") {
            Ok(UnionMemberWrite::Holder { holder, next }) => {
                assert_eq!(holder, "built");
                assert_eq!(next("7"), "SceStatic.set(built, 'radius', 7)");
            }
            Ok(UnionMemberWrite::Place(place)) => {
                panic!("an Interpreter union has no place: {place}")
            }
            Err(why) => panic!("{why}"),
        }
        assert!(JsComposed
            .union_member_write("built", union, &all, "kind")
            .is_err());
        assert!(JsComposed
            .union_member_write("built", union, &all, "radius")
            .is_err());
    }
}
