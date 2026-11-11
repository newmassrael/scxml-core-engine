// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C11 spelling of the walks over a composed value ([`super`])
//! (docs/adr/0086, section 7.1, the C11 row), with no allocator and no
//! closure.
//!
//! | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record |
//! |---|---|---|---|
//! | `{ size_t len; T data[N]; }` | `sce_static_string_<N>_t` / `sce_static_bytes_<N>_t` | `{ bool present; T value; }` | the machine's struct |
//!
//! Every composition is one named type, its name made of its parts and its
//! bound and its declaration guarded by that name (section 7.2), so a place
//! that holds a value of one shape, a variable, a record, a payload or a part
//! of one at any depth, holds it in one type, and a value copies by
//! assignment, as a C struct that holds an array does. A list's members are
//! the library's views' (`len`, `data`), so an expression reads its length and
//! its elements as it reads a list variable's. The machine's storage is the
//! compiler's `sizeof` of these layouts, known at build time.
//!
//! **Readers.** C has no closure to hand a reader of a part to the reader of
//! the part that holds it, so a reader of a value that holds parts is a
//! function of the machine, one per type ([`C11Composed::take_readers`]),
//! which reads the value's text at `sce_value` into the place `sce_out` its
//! type names through the runtime's bounded readers (`sce/event_payload.h`),
//! and answers the sentence of a refusal, or NULL. A reader text the walk
//! composes is an expression of that sentence over the value at the reader's
//! binding (`sce_at`), the place `sce_into` (a `void *`, which the function's
//! typed parameter takes as it is), and the path `sce_path` the caller holds,
//! which each reader extends by the part it reads and leaves at the refused
//! one. Every bound is the type's own: a list past its bound, a text or a
//! byte string past its bound, is refused whole before anything of it is
//! kept, and nothing is cut.
//!
//! **Writers.** A writer is the statements that write a value bare where the
//! wire writer `sce_wire` (a `sce_forge_wire_t *`, `sce/forge/wire.h`)
//! stands, part by part: a scalar is a value, a list and a record a container
//! of parts. An element and a present value are bound to `sce_item`, a
//! pointer to it, in a block of their own, through a pointer named apart
//! first so the binding never reads itself. A host builds a payload by hand,
//! so each writer holds the value to its type as it writes it: a list past
//! the room of its type, a text with no end in its buffer, a byte string past
//! its buffer and an enum value no variant names each refuse the object
//! whole (section 5.4, `sce_forge_wire_refuse`).

use std::cell::RefCell;
use std::collections::BTreeSet;

use super::*;

/// The name a C11 reader binds the text of a part to: the element a list's
/// cursor hands out, a record's field, the present value of one that may be
/// absent.
const C11_READ_BINDING: &str = "sce_at";

/// The place a C11 reader fills, a `void *` to the part's own type.
const C11_INTO: &str = "sce_into";

/// The path a C11 reader names the place it refused by, and the room of it.
const C11_PATH: &str = "sce_path";
const C11_PATH_CAP: &str = "sce_path_cap";

/// The wire writer a C11 writer writes through, a `sce_forge_wire_t *`.
pub(crate) const C11_WIRE: &str = "sce_wire";

/// The name a C11 writer binds a pointer to an element or a present value to.
const C11_WRITE_BINDING: &str = "sce_item";

/// The name a C11 copy binds a pointer to an element or a present value to.
const C11_COPY_BINDING: &str = "sce_copied";

/// Why a C11 walk meets no union: the one decision of what is lowered refuses a
/// union on C11 before any walk reaches one
/// ([`crate::forge::event_schema_check::ComposedLowering::unions`]).
const C11_HOLDS_NO_UNION: &str = "a C11 machine holds no union yet (docs/adr/0092, section 7): \
     the one decision of what is lowered refuses a union on C11 before any walk reaches one";

/// Why a C11 copy meets no value whose parts change their type: a C11 machine
/// takes a composed value whole only into a list variable, whose element is a
/// number, a `bool` or a record of entries (docs/SCE_ACCEPTED_SUBSET.md section
/// 2.15), each of one type in the list it is copied from and the one it is
/// copied into.
const C11_COPIES_ELEMENTS_AS_THEY_ARE: &str =
    "a C11 machine takes a list whole only into a list variable, whose element is a number, a \
     bool or a record of entries, of one type on both sides of the copy";

/// The declarations and reader functions a C11 walk reached, each once, in the
/// order it reached them, an inner part's before the outer one that names it.
#[derive(Debug, Default)]
struct Reached {
    /// Each type a part is held in, by its name, with its declaration.
    types: Vec<String>,
    type_names: BTreeSet<String>,
    types_taken: usize,
    /// Each reader function, by its name, with its definition.
    readers: Vec<String>,
    reader_names: BTreeSet<String>,
    readers_taken: usize,
}

/// The C11 spellings of the walks, for the machine `machine`, whose symbols
/// carry its suite prefix (section 7.1, the C11 row).
///
/// What a walk reaches is declared once by the machine: the types of the
/// parts ([`Self::take_declarations`], which a record's declaration takes
/// first, [`ComposedSpelling::record_def`]) and the reader functions
/// ([`Self::take_readers`]). One spelling walks every payload of a machine, so
/// a type or a reader two payloads share is declared once.
pub struct C11Composed<'a> {
    pub machine: &'a str,
    /// The type each enum the machine holds is declared as (`<document>_enum_t`,
    /// [`crate::forge::static_lowering::StaticTarget::enum_type`]), by the name
    /// the statechart holds it by: what an enum's declared name is written by.
    enums: BTreeMap<String, String>,
    reached: RefCell<Reached>,
}

impl<'a> C11Composed<'a> {
    /// The spelling of the machine `machine`, which holds each enum of `enums`
    /// (by the statechart's name for it) in the type it maps it to.
    pub fn new(machine: &'a str, enums: BTreeMap<String, String>) -> Self {
        Self {
            machine,
            enums,
            reached: RefCell::new(Reached::default()),
        }
    }

    /// The declarations of the types the walks reached since the last call,
    /// an inner one first, each under the guard of its name.
    pub fn take_declarations(&self) -> Vec<String> {
        let mut reached = self.reached.borrow_mut();
        let taken = reached.types_taken;
        reached.types_taken = reached.types.len();
        reached.types[taken..].to_vec()
    }

    /// The reader functions the walks reached since the last call, a reader of
    /// a part before the reader of the value that holds it.
    pub fn take_readers(&self) -> Vec<String> {
        let mut reached = self.reached.borrow_mut();
        let taken = reached.readers_taken;
        reached.readers_taken = reached.readers.len();
        reached.readers[taken..].to_vec()
    }

    /// Note the declaration `def` of the type `name`, once.
    fn declare(&self, name: &str, def: impl FnOnce() -> String) {
        if self.reached.borrow().type_names.contains(name) {
            return;
        }
        let def = def();
        let mut reached = self.reached.borrow_mut();
        reached.type_names.insert(name.to_string());
        reached.types.push(def);
    }

    /// The call of the reader function of the type `ty` over the value at
    /// `value`, defined once by `body`, the statements of the function over
    /// its parameters `sce_value` and `sce_out`, a `ty *`.
    fn reader_of(
        &self,
        name: &str,
        ty: &str,
        value: &str,
        body: impl FnOnce() -> String,
    ) -> String {
        let function = format!("sce_composed_read_{name}");
        if !self.reached.borrow().reader_names.contains(&function) {
            let def = format!(
                "/* docs/adr/0086, section 4: a value of `{ty}` read out of the text at `sce_value`\n   \
                 into `sce_out`, refused whole, its path left in `{C11_PATH}`, when it is outside its\n   \
                 type. */\n\
                 SCE_C_UNUSED static const char *{function}(const char *sce_value, {ty} *sce_out, char \
                 *{C11_PATH},\n        size_t {C11_PATH_CAP}) {{\n    (void){C11_PATH};\n    \
                 (void){C11_PATH_CAP};\n{}}}\n",
                body()
            );
            let mut reached = self.reached.borrow_mut();
            reached.reader_names.insert(function.clone());
            reached.readers.push(def);
        }
        format!("{function}({value}, {C11_INTO}, {C11_PATH}, {C11_PATH_CAP})")
    }

    /// The statement that writes the JSON value `value`, a `sce_forge_wire_value_t`,
    /// where the writer stands.
    fn put(value: &str) -> String {
        format!("sce_forge_wire_put_value({C11_WIRE}, {value});\n")
    }
}

/// The value `held` is, or the one it points to: what a writer and a copy
/// read a part through.
fn value_of(held: &Held) -> String {
    if held.by_reference {
        format!("(*{})", held.text)
    } else {
        format!("({})", held.text)
    }
}

/// `name` without the `_t` its type name ends in: what the name of a function
/// of the type is made of.
fn stem(ty: &str) -> &str {
    ty.trim_end_matches("_t")
}

/// The JSON text of the key `id`, quoted and escaped, as a C string literal's
/// body, and its length in bytes with the colon after it.
fn c11_key(id: &str) -> (String, usize) {
    let json = format!("\"{}\"", crate::filters::escape_json_string(id.to_string()));
    let len = json.len() + 1;
    (crate::filters::escape_c(json), len)
}

/// The statements that write each of `fields`, each its id with the
/// statements of its value ([`super::field_writers`]), as a pair of the top
/// object the wire writer `sce_wire` stands in, its key ascending by its
/// bytes (ARCHITECTURE.md, "JSON Object Key Order", at every depth): what the
/// inject seam writes a payload as.
pub(crate) fn c11_pairs(fields: &[(String, String)]) -> String {
    // A record's ids are distinct, so the pairs order by their ids.
    let mut sorted: Vec<&(String, String)> = fields.iter().collect();
    sorted.sort();
    sorted
        .into_iter()
        .map(|(id, written)| {
            let (key, _) = c11_key(id);
            format!("sce_forge_wire_pair_key({C11_WIRE}, \"{key}\");\n{written}")
        })
        .collect()
}

/// The most bytes of the path below a value of `ty`, written in the scope
/// `within` names, that a reader of it extends the path by (docs/adr/0086,
/// section 4.2): a `/` and the field's id for each record on the way, a `/`
/// and the digits of the last index for each list, nothing for an optional
/// value, whose present value stands where it does. What the buffer a C11
/// lift names a refused place in is sized by. `Err` names a part no C11 walk
/// reads (a union, a record the statechart does not hold).
pub(crate) fn c11_path_len(
    ty: &ValueType,
    within: Option<&str>,
    records: &dyn RecordEnv,
) -> Result<u64, String> {
    Ok(match ty {
        ValueType::Scalar(_) | ValueType::Text { .. } | ValueType::Bytes { .. } => 0,
        ValueType::Optional { elem } => c11_path_len(elem, within, records)?,
        ValueType::List { elem, max } => {
            let last = max.max.saturating_sub(1);
            1 + last.to_string().len() as u64 + c11_path_len(elem, within, records)?
        }
        ValueType::Record { alias } => {
            let held = records.record(within, alias)?;
            let mut longest = 0;
            for field in &held.schema.fields {
                let below = c11_path_len(&field.sce_type, Some(held.alias), records)?;
                longest = longest.max(1 + field.id.len() as u64 + below);
            }
            longest
        }
        ValueType::Union { .. } => return Err(C11_HOLDS_NO_UNION.to_string()),
    })
}

impl ComposedSpelling for C11Composed<'_> {
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::c_type(ty).to_string()
    }
    // The buffer of the bound, which a variable and a record's field of that
    // bound hold a text in too, declared under its guard.
    fn text_type(&self, max: u32) -> String {
        let ty = crate::forge::static_lowering::c_string_storage_type(max);
        self.declare(&ty, || {
            crate::forge::static_lowering::c_string_storage_def(max)
        });
        ty
    }
    fn bytes_type(&self, max: u32) -> String {
        let ty = crate::forge::static_lowering::c_bytes_storage_type(max);
        self.declare(&ty, || {
            crate::forge::static_lowering::c_bytes_storage_def(max)
        });
        ty
    }
    // The one struct of a flag and a value (`sce/forge/optional.h`): the
    // runtime's own for a number or a `bool`, and one declared beside its
    // element for a text, a byte string, an enum or a struct.
    fn optional_type(&self, elem: &str) -> String {
        use crate::forge::static_lowering as lowering;
        let Some(name) = lowering::c11_optional_elem(elem) else {
            unreachable!("every C11 type a part is held in ends in `_t` or is a scalar: `{elem}`")
        };
        let ty = format!("sce_optional_{name}_t");
        if lowering::c11_scalar_word(elem).is_none() {
            self.declare(&ty, || {
                lowering::c11_optional_declaration(elem)
                    .unwrap_or_else(|| lowering::c11_optional_record_declaration(&name, elem))
            });
        }
        ty
    }
    fn list_type(&self, elem: &str, max: u32) -> String {
        use crate::forge::static_lowering as lowering;
        let ty = lowering::c11_list_type(elem, max);
        self.declare(&ty, || {
            lowering::c11_list_def(
                elem,
                max,
                &format!("docs/adr/0086, section 7.2: a list of at most {max} of `{elem}`."),
            )
        });
        ty
    }
    // The name a statechart's record variable of the same schema is held in
    // (`StaticTarget::record_type`), so a record a payload holds is the type a
    // variable holds.
    fn record_type(&self, alias: &str) -> String {
        crate::forge::static_lowering::c11_record_type(self.machine, alias)
    }
    // Plain data by the record rule: a struct of the schema's fields in the
    // schema's order, each named as the schema names it, which is how a read
    // names it, after the types of its fields. A C struct has no constructor
    // that could refuse a value, so a writer holds a host's value to its type
    // as it writes it, and no check is declared beside it.
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        fields: &[(String, String)],
        _checks: &[String],
    ) -> String {
        let members: String = fields
            .iter()
            .map(|(id, ty)| format!("    {ty} {id};\n"))
            .collect();
        let mut parts = self.take_declarations();
        parts.push(format!(
            "/* docs/adr/0086, section 7.2: a `record:{alias}` a composed value holds. */\n\
             typedef struct {{\n{members}}} {ty};"
        ));
        parts.join("\n\n")
    }

    fn read_binding(&self) -> &'static str {
        C11_READ_BINDING
    }
    // The runtime's reader of the width (`sce/event_payload.h`), which refuses a
    // value the width cannot hold.
    fn read_scalar(&self, ty: &SceType, value: &str) -> String {
        format!(
            "sce_payload_value_{}({value}, {C11_INTO})",
            crate::forge::generator::c11_payload_value_suffix(ty)
        )
    }
    // By the variant's declared name, against the names the enum declares, in
    // place: a name it does not declare is refused.
    fn read_enum(&self, held: &MachineEnum, value: &str) -> String {
        let names: String = held
            .variants
            .iter()
            .map(|(declared, _)| format!("\"{}\", ", crate::filters::escape_c(declared.clone())))
            .collect();
        let values: String = held
            .variants
            .iter()
            .map(|(_, ident)| format!("{ident}, "))
            .collect();
        let ty = &held.ty;
        self.reader_of(stem(ty), ty, value, || {
            format!(
                "    static const char *const sce_names[] = {{ {names}}};\n    \
                 static const {ty} sce_values[] = {{ {values}}};\n    \
                 size_t sce_index = 0u;\n    \
                 const char *const sce_refusal =\n        \
                 sce_payload_value_variant(sce_value, sce_names, sizeof(sce_names) / \
                 sizeof(sce_names[0]), &sce_index);\n    \
                 if (sce_refusal != NULL) {{\n        return sce_refusal;\n    }}\n    \
                 *sce_out = sce_values[sce_index];\n    return NULL;\n"
            )
        })
    }
    // Into the buffer of the bound, its terminator after it.
    fn read_text(&self, max: u32, value: &str) -> String {
        let ty = self.text_type(max);
        self.reader_of(stem(&ty), &ty, value, || {
            format!("    return sce_payload_value_text(sce_value, sce_out->data, {max}u);\n")
        })
    }
    fn read_bytes(&self, max: u32, value: &str) -> String {
        let ty = self.bytes_type(max);
        self.reader_of(stem(&ty), &ty, value, || {
            format!(
                "    return sce_payload_value_bytes(sce_value, sce_out->data, {max}u, \
                 &sce_out->len);\n"
            )
        })
    }
    // `null` is absent; any other value is the present value, read as its type
    // is.
    fn read_optional(&self, value: &str, elem: &str, elem_ty: &str) -> String {
        let ty = self.optional_type(elem_ty);
        self.reader_of(stem(&ty), &ty, value, || {
            format!(
                "    if (sce_payload_is_null(sce_value)) {{\n        sce_out->present = false;\n        \
                 return NULL;\n    }}\n    {{\n        \
                 const char *const {C11_READ_BINDING} = sce_value;\n        \
                 void *const {C11_INTO} = &sce_out->value;\n        \
                 sce_out->present = true;\n        return {elem};\n    }}\n"
            )
        })
    }
    // The elements are counted when the list is opened, and a list past its
    // bound or short of its lower count is refused before any is read (section
    // 15.5); each element is read into the next slot, at its index's path.
    fn read_list(&self, value: &str, least: u32, max: u32, elem: &str, elem_ty: &str) -> String {
        let ty = self.list_type(elem_ty, max);
        let name = if least == 0 {
            stem(&ty).to_string()
        } else {
            format!("{}_from_{least}", stem(&ty))
        };
        self.reader_of(&name, &ty, value, || {
            format!(
                "    sce_payload_list_t sce_cursor;\n    \
                 const char *{C11_READ_BINDING} = NULL;\n    \
                 const char *sce_refusal = sce_payload_list_open_within(sce_value, {least}u, {max}u, \
                 &sce_cursor);\n    \
                 const size_t sce_head = strlen({C11_PATH});\n    \
                 sce_out->len = 0u;\n    \
                 if (sce_refusal != NULL) {{\n        return sce_refusal;\n    }}\n    \
                 while (sce_payload_list_next(&sce_cursor, &{C11_READ_BINDING})) {{\n        \
                 void *const {C11_INTO} = &sce_out->data[sce_out->len];\n        \
                 (void)sce_payload_path_index({C11_PATH}, {C11_PATH_CAP}, {C11_PATH}, \
                 (uint32_t)sce_out->len);\n        \
                 sce_refusal = {elem};\n        \
                 if (sce_refusal != NULL) {{\n            return sce_refusal;\n        }}\n        \
                 {C11_PATH}[sce_head] = '\\0';\n        \
                 sce_out->len++;\n    }}\n    return NULL;\n"
            )
        })
    }
    // Each field read out of the object by its name, a field a later revision
    // added out of the schema's defaults where the value lacks it, at its own
    // path; a key the schema does not declare is never asked for.
    fn read_record(
        &self,
        value: &str,
        defaults: Option<&str>,
        ty: &str,
        fields: &[(String, String)],
    ) -> String {
        let defaults = defaults.map_or_else(
            || "NULL".to_string(),
            |json| format!("\"{}\"", crate::filters::escape_c(json.to_string())),
        );
        let reads: String = fields
            .iter()
            .map(|(id, read)| {
                let id = crate::filters::escape_c(id.clone());
                format!(
                    "    {{\n        const char *{C11_READ_BINDING} = NULL;\n        \
                     void *const {C11_INTO} = &sce_out->{id};\n        \
                     (void)sce_payload_path_join({C11_PATH}, {C11_PATH_CAP}, {C11_PATH}, \"{id}\");\n        \
                     sce_refusal = sce_payload_field(&sce_fields, \"{id}\", &{C11_READ_BINDING});\n        \
                     if (sce_refusal == NULL) {{\n            sce_refusal = {read};\n        }}\n        \
                     if (sce_refusal != NULL) {{\n            return sce_refusal;\n        }}\n        \
                     {C11_PATH}[sce_head] = '\\0';\n    }}\n"
                )
            })
            .collect();
        self.reader_of(stem(ty), ty, value, || {
            format!(
                "    sce_payload_fields_t sce_fields;\n    \
                 const char *sce_refusal = sce_payload_record_open(sce_value, {defaults}, &sce_fields);\n    \
                 const size_t sce_head = strlen({C11_PATH});\n    \
                 if (sce_refusal != NULL) {{\n        return sce_refusal;\n    }}\n\
                 {reads}    return NULL;\n"
            )
        })
    }

    fn write_binding(&self) -> &'static str {
        C11_WRITE_BINDING
    }
    // As a scalar param crosses (`StaticTarget::wire_value`): an integer of at
    // most 32 bits widened to 64 and a `float32` to `float64`, both exactly, and
    // a 64-bit integer as the text of its digits.
    fn write_scalar(&self, ty: &SceType, held: &Held) -> String {
        let value = value_of(held);
        let wired = match ty {
            SceType::Bool => format!("sce_forge_wire_bool({value})"),
            SceType::Int64 => format!("sce_forge_wire_wide_int((int64_t){value})"),
            SceType::Uint64 => format!("sce_forge_wire_wide_uint((uint64_t){value})"),
            SceType::Int8 | SceType::Int16 | SceType::Int32 => {
                format!("sce_forge_wire_int((int64_t){value})")
            }
            SceType::Uint8 | SceType::Uint16 | SceType::Uint32 => {
                format!("sce_forge_wire_uint((uint64_t){value})")
            }
            SceType::Float32 | SceType::Float64 => format!("sce_forge_wire_real((double){value})"),
            SceType::String | SceType::Bytes | SceType::Enum(_) => unreachable!(
                "a text, a byte string and an enum are each written by a writer of their own"
            ),
        };
        Self::put(&wired)
    }
    fn write_enum(&self, _held: &Held) -> String {
        unreachable!(
            "a C11 value of an enum is written by the enum's own function of its declared names, \
             which the walk names by its alias (`write_enum_of`)"
        )
    }
    // The name the enum's document declares for the value
    // (`<document>_declared_name`, `StaticTarget::enum_def`); a value no
    // variant names refuses the object.
    fn write_enum_of(&self, alias: &str, held: &Held) -> String {
        let ty = self.enums.get(alias).unwrap_or_else(|| {
            panic!(
                "`enum:{alias}` reached the C11 writer of a machine that holds no type for it: the \
                 static lowering declares every enum a composed value names before it is written"
            )
        });
        let named = format!("{}_declared_name", ty.trim_end_matches("_enum_t"));
        format!(
            "{{\n    const char *const sce_name = {named}({});\n    \
             if (sce_name == NULL) {{\n        sce_forge_wire_refuse({C11_WIRE});\n    }} else {{\n        \
             {}    }}\n}}\n",
            value_of(held),
            Self::put("sce_forge_wire_string(sce_name)")
        )
    }
    // The text up to its terminator, which a buffer a host filled by hand holds
    // inside its bound or not at all.
    fn write_text(&self, held: &Held) -> String {
        let value = value_of(held);
        format!(
            "if (memchr({value}.data, '\\0', sizeof({value}.data)) == NULL) {{\n    \
             sce_forge_wire_refuse({C11_WIRE});\n}} else {{\n    {}}}\n",
            Self::put(&format!("sce_forge_wire_string({value}.data)"))
        )
    }
    // Its byte-exact Latin-1 text, as a byte string param crosses.
    fn write_bytes(&self, held: &Held) -> String {
        let value = value_of(held);
        format!(
            "if ({value}.len > sizeof({value}.data)) {{\n    sce_forge_wire_refuse({C11_WIRE});\n}} \
             else {{\n    {}}}\n",
            Self::put(&format!(
                "sce_forge_wire_bytes((sce_forge_bytes_view_t){{ {value}.data, {value}.len }})"
            ))
        )
    }
    // `null` where it is absent, and its present value where it is present.
    fn write_optional(&self, held: &Held, elem: &str) -> String {
        let value = value_of(held);
        format!(
            "if ({value}.present) {{\n    \
             __typeof__({value}.value) const *const sce_next = &{value}.value;\n    {{\n        \
             __typeof__(sce_next) {C11_WRITE_BINDING} = sce_next;\n{elem}    }}\n}} else {{\n    {}}}\n",
            Self::put("sce_forge_wire_null()")
        )
    }
    // Each element in the list's order, after the count is held to the room
    // of the list's type.
    fn write_list(&self, held: &Held, elem: &str) -> String {
        let value = value_of(held);
        format!(
            "if ({value}.len > sizeof({value}.data) / sizeof({value}.data[0])) {{\n    \
             sce_forge_wire_refuse({C11_WIRE});\n}} else {{\n    \
             sce_forge_wire_open_container({C11_WIRE}, '[');\n    \
             for (size_t sce_i = 0u; sce_i < {value}.len; ++sce_i) {{\n        \
             __typeof__({value}.data[0]) const *const sce_next = &{value}.data[sce_i];\n        \
             sce_forge_wire_next_in_container({C11_WIRE});\n        {{\n            \
             __typeof__(sce_next) {C11_WRITE_BINDING} = sce_next;\n{elem}        }}\n    }}\n    \
             sce_forge_wire_close({C11_WIRE});\n}}\n"
        )
    }
    fn write_field(&self, held: &Held, field: &str) -> String {
        format!("{}.{field}", value_of(held))
    }
    // The count held first, as the list's room is: a list short of its
    // field's lower count refuses the wire before any of it is written.
    fn send_counted(&self, held: &Held, _field: &str, least: u32, sent: String) -> Option<String> {
        Some(format!(
            "if ({}.len < {least}u) {{\n    sce_forge_wire_refuse({C11_WIRE});\n}} else {{\n{sent}}}\n",
            value_of(held)
        ))
    }
    // An object of the fields, its keys ascending by their bytes.
    fn write_record(&self, fields: &[(String, String)]) -> String {
        // A record's ids are distinct, so the members order by their ids.
        let mut sorted: Vec<&(String, String)> = fields.iter().collect();
        sorted.sort();
        let members: String = sorted
            .into_iter()
            .map(|(id, written)| {
                let (key, len) = c11_key(id);
                format!(
                    "sce_forge_wire_next_in_container({C11_WIRE});\n\
                     sce_forge_wire_put({C11_WIRE}, \"{key}:\", {len}u);\n{written}"
                )
            })
            .collect();
        format!(
            "sce_forge_wire_open_container({C11_WIRE}, '{{');\n{members}\
             sce_forge_wire_close({C11_WIRE});\n"
        )
    }

    fn copy_binding(&self) -> &'static str {
        C11_COPY_BINDING
    }
    // A C struct copies by assignment, an array it holds included.
    fn copy_whole(&self, held: &Held, _plain: bool) -> String {
        value_of(held)
    }
    // The buffer of the larger bound, filled from the text, which fits it.
    fn copy_text(&self, max: u32, held: &Held) -> String {
        format!(
            "{}({}.data)",
            crate::forge::static_lowering::c_string_storage_of(max),
            value_of(held)
        )
    }
    fn copy_bytes(&self, max: u32, held: &Held) -> String {
        let value = value_of(held);
        format!(
            "{}((sce_forge_bytes_view_t){{ {value}.data, {value}.len }})",
            crate::forge::static_lowering::c_bytes_storage_of(max)
        )
    }
    fn copy_optional(&self, _held: &Held, _elem: &str, _elem_ty: &str) -> String {
        unreachable!("{C11_COPIES_ELEMENTS_AS_THEY_ARE}")
    }
    fn copy_present(&self, _value: &str) -> String {
        unreachable!("{C11_COPIES_ELEMENTS_AS_THEY_ARE}")
    }
    fn copy_list(&self, _held: &Held, _elem: &str, _elem_ty: &str) -> String {
        unreachable!(
            "a C11 list is copied into the type of the place it is copied into, which the walk \
             names (`copy_list_into`)"
        )
    }
    // The list of the place's own type made of the elements, by the
    // constructor its declaration holds (`c11_list_def`): the elements are of
    // one type on both sides.
    fn copy_list_into(&self, held: &Held, elem: &str, _elem_ty: &str, list_ty: &str) -> String {
        let as_it_is = elem
            .strip_prefix("(*")
            .and_then(|rest| rest.strip_suffix(')'))
            == Some(C11_COPY_BINDING);
        if !as_it_is {
            unreachable!("{C11_COPIES_ELEMENTS_AS_THEY_ARE}: `{elem}`")
        }
        let value = value_of(held);
        format!("{}_of({value}.data, {value}.len)", stem(list_ty))
    }

    // A C11 machine holds no union yet: the gate refuses one by name before a
    // walk reaches it, so each spelling below either names what it would be or
    // says why there is none.
    fn union_type(&self, name: &str) -> String {
        format!(
            "{}_union_{}_t",
            self.machine,
            crate::filters::to_snake_case(name.to_string())
        )
    }
    fn union_case_ident(&self, variant: &str) -> String {
        crate::filters::to_snake_case(variant.to_string())
    }
    fn union_tag_read(&self, _tag: &str) -> Option<String> {
        None
    }
    fn union_case_read(&self, variant: &str) -> String {
        format!(
            "proven_{}",
            crate::filters::to_snake_case(variant.to_string())
        )
    }
    fn union_shared_read(&self, field: &str) -> String {
        format!(
            "proven_field_{}",
            crate::filters::to_snake_case(field.to_string())
        )
    }
    fn union_def(&self, _def: &UnionDef) -> String {
        unreachable!("{C11_HOLDS_NO_UNION}")
    }
    fn union_value(&self, _ty: &str, _tag: &str, _case: &str, _record: Option<&str>) -> String {
        unreachable!("{C11_HOLDS_NO_UNION}")
    }
    fn read_union(&self, _value: &str, _read: &UnionRead<'_>) -> String {
        unreachable!("{C11_HOLDS_NO_UNION}")
    }
    fn write_union(&self, _held: &Held, _ty: &str, _tag: &str, _arms: &[UnionWriteArm]) -> String {
        unreachable!("{C11_HOLDS_NO_UNION}")
    }
    fn union_member(
        &self,
        _union: &ScopedUnion,
        _left: &[&ScopedCase],
        _member: &str,
    ) -> Result<String, String> {
        Err(C11_HOLDS_NO_UNION.to_string())
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
        Err(C11_HOLDS_NO_UNION.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    /// The C11 spelling of the machine `m`, which holds the enum `Level` as
    /// the type its document declares.
    fn c11() -> C11Composed<'static> {
        C11Composed::new(
            "m",
            [("Level".to_string(), "level_enum_t".to_string())].into(),
        )
    }

    /// The enums the walks read, the C11 type of `Level` among them.
    fn c11_enums() -> BTreeMap<String, MachineEnum> {
        [(
            "Level".to_string(),
            MachineEnum {
                alias: "Level".to_string(),
                ty: "level_enum_t".to_string(),
                variants: vec![
                    ("low".to_string(), "LEVEL_LOW".to_string()),
                    ("high".to_string(), "LEVEL_HIGH".to_string()),
                ],
            },
        )]
        .into()
    }

    /// The C11 row of the one table of what each backend lowers of a composed
    /// value: a payload read at any depth, a list taken whole and a composed
    /// value sent (docs/adr/0086, sections 5 and 6), and no union held yet
    /// (docs/adr/0092, section 7).
    #[test]
    fn the_c11_row_lowers_every_composed_position_but_a_union() {
        use crate::forge::event_schema_check::ComposedLowering;
        assert_eq!(
            ComposedLowering::of(crate::generator::Language::C11),
            ComposedLowering {
                name: "c11",
                reads: true,
                copies: true,
                unions: false,
                records: false,
                sends: true,
            }
        );
    }

    /// The C11 row of section 7.1: every composition a named type of its
    /// parts and bound, a list the library's view of its elements, a value
    /// that may be absent the one struct of a flag and a value, a record the
    /// machine's struct after the types of its fields, each type declared once
    /// under its guard.
    #[test]
    fn a_c11_machine_holds_each_constructor_in_a_type_named_by_its_shape() {
        let held = held();
        let enums = c11_enums();
        let spelling = c11();
        let spelled =
            |text: &str| machine_type(&ty(text), None, &held, &enums, &spelling).expect("a type");
        assert_eq!(spelled("uint16"), "uint16_t");
        assert_eq!(spelled("string[8]"), "sce_static_string_8_t");
        assert_eq!(spelled("bytes[4]"), "sce_static_bytes_4_t");
        assert_eq!(spelled("optional<int64>"), "sce_optional_int64_t");
        assert_eq!(
            spelled("list<list<uint8>[2]>[3]"),
            "sce_static_list_uint8_2_list_3_t"
        );
        assert_eq!(spelled("list<enum:Level>[2]"), "level_enum_list_2_t");
        assert_eq!(spelled("list<record:Item>[3]"), "m_record_item_list_3_t");
        assert_eq!(
            spelled("optional<list<uint8>[2]>"),
            "sce_optional_record_sce_static_list_uint8_2_t"
        );
        assert_eq!(
            spelled("list<optional<uint16>>[2]"),
            "sce_optional_uint16_list_2_t"
        );
        let declared = spelling.take_declarations().join("\n");
        // Each type once, under the guard of its name, an inner one first.
        assert_eq!(
            declared
                .matches("#ifndef SCE_STATIC_LIST_UINT8_2\n")
                .count(),
            1,
            "{declared}"
        );
        let inner = declared
            .find("} sce_static_list_uint8_2_t;")
            .expect("the inner list is declared");
        let outer = declared
            .find("} sce_static_list_uint8_2_list_3_t;")
            .expect("the outer list is declared");
        assert!(inner < outer, "{declared}");
        assert!(
            declared.contains("    size_t len;\n    sce_static_list_uint8_2_t data[3];\n"),
            "{declared}"
        );
        assert!(
            declared.contains(
                "SCE_OPTIONAL_DECLARE_RECORD(record_sce_static_list_uint8_2, \
                 sce_static_list_uint8_2_t)"
            ),
            "{declared}"
        );
        assert!(
            spelling.take_declarations().is_empty(),
            "what was taken is not taken again"
        );
        let members = record_members("Item", &held, &enums, &spelling).expect("members");
        let def = spelling.record_def("m_record_item_t", "Item", &members, &[]);
        assert!(
            def.ends_with(
                "typedef struct {\n    uint32_t id;\n    level_enum_t level;\n    \
                 sce_optional_int64_t due;\n    sce_static_bytes_4_t blob;\n    \
                 sce_static_string_8_list_4_t tags;\n} m_record_item_t;"
            ),
            "{def}"
        );
        // The types its fields are held in come before it.
        assert!(
            def.find("} sce_static_string_8_list_4_t;") < def.find("} m_record_item_t;"),
            "{def}"
        );
    }

    /// A scalar read by the runtime's reader of its width; every other part by
    /// a function of the machine per type, defined once, a part's before the
    /// one that holds it, each bound and lower count handed to the cursor that
    /// holds it before any element is read.
    #[test]
    fn a_c11_reader_is_one_function_per_type_over_the_runtime_readers() {
        let held = held();
        let enums = c11_enums();
        let spelling = c11();
        let read =
            |text: &str| reader(&ty(text), None, &held, &enums, &spelling, "v").expect("read");
        assert_eq!(read("uint8"), "sce_payload_value_u8(v, sce_into)");
        assert_eq!(read("int64"), "sce_payload_value_i64(v, sce_into)");
        assert_eq!(read("float32"), "sce_payload_value_f32(v, sce_into)");
        assert_eq!(read("bool"), "sce_payload_value_bool(v, sce_into)");
        assert_eq!(
            read("list<list<uint8>[2]>[3]"),
            "sce_composed_read_sce_static_list_uint8_2_list_3(v, sce_into, sce_path, sce_path_cap)"
        );
        let readers = spelling.take_readers();
        assert_eq!(readers.len(), 2, "{readers:#?}");
        assert!(
            readers[0].contains("sce_composed_read_sce_static_list_uint8_2(const char *sce_value")
                && readers[0]
                    .contains("sce_payload_list_open_within(sce_value, 0u, 2u, &sce_cursor)")
                && readers[0].contains("sce_refusal = sce_payload_value_u8(sce_at, sce_into);"),
            "{}",
            readers[0]
        );
        assert!(
            readers[1].contains("sce_payload_list_open_within(sce_value, 0u, 3u, &sce_cursor)")
                && readers[1].contains(
                    "sce_refusal = sce_composed_read_sce_static_list_uint8_2(sce_at, sce_into, \
                     sce_path, sce_path_cap);"
                )
                && readers[1].contains(
                    "(void)sce_payload_path_index(sce_path, sce_path_cap, sce_path, \
                     (uint32_t)sce_out->len);"
                ),
            "{}",
            readers[1]
        );
        // A reader of a type already read is not defined again.
        let _ = read("list<uint8>[2]");
        assert!(spelling.take_readers().is_empty());
        // A list that states its lower count reads with it, by a reader of its own.
        assert_eq!(
            read("list<uint8>[1..2]"),
            "sce_composed_read_sce_static_list_uint8_2_from_1(v, sce_into, sce_path, \
             sce_path_cap)"
        );
        assert!(spelling.take_readers()[0]
            .contains("sce_payload_list_open_within(sce_value, 1u, 2u, &sce_cursor)"));
        // A text and a byte string into the buffer of their bound, held to it.
        let _ = read("string[8]");
        let _ = read("bytes[4]");
        let readers = spelling.take_readers().join("\n");
        assert!(
            readers.contains("return sce_payload_value_text(sce_value, sce_out->data, 8u);")
                && readers.contains(
                    "return sce_payload_value_bytes(sce_value, sce_out->data, 4u, &sce_out->len);"
                ),
            "{readers}"
        );
        // An enum by the names its document declares.
        let _ = read("enum:Level");
        let level = spelling.take_readers().join("\n");
        assert!(
            level.contains("static const char *const sce_names[] = { \"low\", \"high\", };")
                && level.contains(
                    "static const level_enum_t sce_values[] = { LEVEL_LOW, LEVEL_HIGH, };"
                ),
            "{level}"
        );
        // A record field by field at each field's path, with its schema's
        // defaults, and a value that may be absent as `null` or its value.
        let _ = read("record:Batch");
        let batch = spelling.take_readers().join("\n");
        assert!(
            batch
                .contains("sce_payload_record_open(sce_value, \"{\\\"late\\\":[]}\", &sce_fields)")
                && batch.contains(
                    "(void)sce_payload_path_join(sce_path, sce_path_cap, sce_path, \"items\");"
                )
                && batch
                    .contains("sce_refusal = sce_payload_field(&sce_fields, \"due\", &sce_at);")
                && batch.contains("if (sce_payload_is_null(sce_value)) {"),
            "{batch}"
        );
    }

    /// A writer writes each part bare where the wire writer stands, a list's
    /// count and a text's end held to the room of its type, a record's keys
    /// ascending, an enum by its declared name or the object refused.
    #[test]
    fn a_c11_writer_writes_each_part_where_it_stands() {
        let held = held();
        let spelling = c11();
        let write = |text: &str| {
            writer(&ty(text), None, &held, &spelling, &Held::place("p")).expect("a writer")
        };
        assert_eq!(
            write("uint8"),
            "sce_forge_wire_put_value(sce_wire, sce_forge_wire_uint((uint64_t)(p)));\n"
        );
        assert_eq!(
            write("int64"),
            "sce_forge_wire_put_value(sce_wire, sce_forge_wire_wide_int((int64_t)(p)));\n"
        );
        let list = write("list<uint8>[2]");
        assert!(
            list.starts_with(
                "if ((p).len > sizeof((p).data) / sizeof((p).data[0])) {\n    \
                 sce_forge_wire_refuse(sce_wire);\n}"
            ) && list.contains("sce_forge_wire_open_container(sce_wire, '[');")
                && list.contains(
                    "sce_forge_wire_put_value(sce_wire, sce_forge_wire_uint((uint64_t)(*sce_item)));"
                )
                && list.contains("sce_forge_wire_close(sce_wire);"),
            "{list}"
        );
        let text = write("string[8]");
        assert!(
            text.contains("if (memchr((p).data, '\\0', sizeof((p).data)) == NULL) {")
                && text.contains("sce_forge_wire_string((p).data)"),
            "{text}"
        );
        let level = write("enum:Level");
        assert!(
            level.contains("const char *const sce_name = level_declared_name((p));")
                && level
                    .contains("if (sce_name == NULL) {\n        sce_forge_wire_refuse(sce_wire);"),
            "{level}"
        );
        let optional = write("optional<int64>");
        assert!(
            optional.starts_with("if ((p).present) {")
                && optional.contains("sce_forge_wire_put_value(sce_wire, sce_forge_wire_null());"),
            "{optional}"
        );
        // Item's keys ascending by their bytes: blob, due, id, level, tags.
        let item = write("record:Item");
        let at = |key: &str| {
            item.find(&format!("sce_forge_wire_put(sce_wire, \"\\\"{key}\\\":\""))
                .unwrap_or_else(|| panic!("`{key}` in {item}"))
        };
        assert!(
            at("blob") < at("due")
                && at("due") < at("id")
                && at("id") < at("level")
                && at("level") < at("tags"),
            "{item}"
        );
        assert!(
            item.contains("sce_forge_wire_put(sce_wire, \"\\\"tags\\\":\", 7u);"),
            "{item}"
        );
        // The pairs of a payload, its keys ascending, each named first.
        let pairs = c11_pairs(&[
            ("b".to_string(), "B;\n".to_string()),
            ("a".to_string(), "A;\n".to_string()),
        ]);
        assert_eq!(
            pairs,
            "sce_forge_wire_pair_key(sce_wire, \"\\\"a\\\"\");\nA;\n\
             sce_forge_wire_pair_key(sce_wire, \"\\\"b\\\"\");\nB;\n"
        );
    }

    /// A list taken whole into a place of the same type is the value as it is;
    /// into one of a larger bound, the place's own type made of its elements.
    #[test]
    fn a_c11_list_is_copied_into_the_type_of_its_place() {
        let held = held();
        let enums = c11_enums();
        let spelling = c11();
        let copy = |from: &str, to: &str| {
            copier(
                &ty(from),
                None,
                &ty(to),
                None,
                &held,
                &enums,
                &spelling,
                &Held::place("p"),
            )
            .expect("a copy")
        };
        assert_eq!(copy("list<uint8>[2]", "list<uint8>[2]"), "(p)");
        assert_eq!(
            copy("list<uint8>[2]", "list<uint8>[4]"),
            "sce_static_list_uint8_4_of((p).data, (p).len)"
        );
        assert_eq!(
            copy("list<record:Item>[2]", "list<record:Item>[3]"),
            "m_record_item_list_3_of((p).data, (p).len)"
        );
    }

    /// The path a refusal names a place by is as long as the longest path a
    /// value of the type has: a `/` and an id per record, a `/` and the digits
    /// of the last index per list, nothing for a value that may be absent.
    #[test]
    fn a_c11_path_is_as_long_as_the_longest_place_of_its_type() {
        let held = held();
        let len = |text: &str| c11_path_len(&ty(text), None, &held).expect("a length");
        assert_eq!(len("uint8"), 0);
        assert_eq!(len("optional<int64>"), 0);
        assert_eq!(len("list<uint8>[10]"), 2);
        assert_eq!(len("list<uint8>[11]"), 3);
        // `/tags/3` below an item.
        assert_eq!(len("record:Item"), 7);
        // `/items/2/tags/3`, the longest below a batch.
        assert_eq!(len("record:Batch"), 15);
        assert!(c11_path_len(&ty("union:Shape"), None, &held).is_err());
    }

    /// A list sent where its field asks a lower count it does not prove is
    /// held to the count before any of it is written (docs/adr/0086, section
    /// 15.6), the wire refused as the list's room refuses it.
    #[test]
    fn a_list_sent_short_of_its_fields_count_refuses_the_wire() {
        assert_eq!(
            c11().send_counted(&Held::place("sm->taking"), "ops", 1, "W;\n".to_string()),
            Some(
                "if ((sm->taking).len < 1u) {\n    sce_forge_wire_refuse(sce_wire);\n} else {\nW;\n}\n"
                    .to_string()
            )
        );
    }
}
