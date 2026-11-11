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
pub(crate) const C11_INTO: &str = "sce_into";

/// The path a C11 reader names the place it refused by, and the room of it.
pub(crate) const C11_PATH: &str = "sce_path";
pub(crate) const C11_PATH_CAP: &str = "sce_path_cap";

/// The place a C11 reader writes the bound a refused value broke into, a
/// `uint32_t *`: a text's or a byte string's bound for `tooLong`, a list's bound
/// for `tooMany` and its lower count for `tooFew` (docs/adr/0087, section 2.4),
/// written only where the reader refuses. A lift names it in its refusal
/// record; a delivery's reason names the path alone.
pub(crate) const C11_BOUND: &str = "sce_bound";

/// The member of a C11 union that holds its tag, a value of the machine's enum
/// of it, whatever key the tag is written under (docs/adr/0092, section 7, the
/// C11 row: `.tag`).
pub(crate) const C11_UNION_TAG: &str = "tag";

/// The member of a C11 union that holds the record of its case, a C `union` of
/// one member per case that holds a record (section 7, the C11 row: `.as`).
pub(crate) const C11_UNION_CASES: &str = "as";

/// The wire writer a C11 writer writes through, a `sce_forge_wire_t *`.
pub(crate) const C11_WIRE: &str = "sce_wire";

/// The name a C11 writer binds a pointer to an element or a present value to.
const C11_WRITE_BINDING: &str = "sce_item";

/// The name a C11 copy binds a pointer to an element or a present value to.
const C11_COPY_BINDING: &str = "sce_copied";

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
    ///
    /// The definition stands under a guard of its name, as a type's does, so a
    /// machine whose payload channel and whose `<sce:lift>` each read a value of
    /// one type, each through a spelling of its own, defines its reader once.
    fn reader_of(
        &self,
        name: &str,
        ty: &str,
        value: &str,
        body: impl FnOnce() -> String,
    ) -> String {
        let function = format!("sce_composed_read_{name}");
        if !self.reached.borrow().reader_names.contains(&function) {
            let guard = function.to_uppercase();
            let def = format!(
                "#ifndef {guard}\n#define {guard}\n\
                 /* docs/adr/0086, section 4: a value of `{ty}` read out of the text at `sce_value`\n   \
                 into `sce_out`, refused whole, its path left in `{C11_PATH}` and the bound it broke\n   \
                 in `*{C11_BOUND}`, when it is outside its type. */\n\
                 SCE_C_UNUSED static const char *{function}(const char *sce_value, {ty} *sce_out, char \
                 *{C11_PATH},\n        size_t {C11_PATH_CAP}, uint32_t *{C11_BOUND}) {{\n    \
                 (void){C11_PATH};\n    (void){C11_PATH_CAP};\n    (void){C11_BOUND};\n{}}}\n#endif\n",
                body()
            );
            let mut reached = self.reached.borrow_mut();
            reached.reader_names.insert(function.clone());
            reached.readers.push(def);
        }
        format!("{function}({value}, {C11_INTO}, {C11_PATH}, {C11_PATH_CAP}, {C11_BOUND})")
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

/// The statements of a reader whose one refusal names `bound`: the refusal of
/// the runtime's `read`, the bound written where it refuses
/// ([`C11_BOUND`]).
fn refused_at_bound(read: &str, bound: u32) -> String {
    format!(
        "    const char *const sce_refusal = {read};\n    if (sce_refusal != NULL) {{\n        \
         *{C11_BOUND} = {bound}u;\n    }}\n    return sce_refusal;\n"
    )
}

/// The identifier of the case `variant` of a C11 union: its member of the
/// union's `as`, named as the variant in snake case (docs/adr/0092, section 7
/// item 1), on a machine's union as on a unit's.
pub(crate) fn c11_case_ident(variant: &str) -> String {
    crate::filters::to_snake_case(variant.to_string())
}

/// The read of the record of the case `variant` where a test proved it, after
/// the union's value: its member of `as` (section 7, the C11 row).
fn c11_case_read(variant: &str) -> String {
    format!("{C11_UNION_CASES}.{}", c11_case_ident(variant))
}

/// The name of the accessor a field several cases share is written through,
/// after the union's type ([`SharedFieldDef`]): a function of the union's type
/// whose other cases end the process, as a C++ and a Go union's are.
fn c11_shared_read(field: &str) -> String {
    format!(
        "proven_field_{}",
        crate::filters::to_snake_case(field.to_string())
    )
}

/// What follows a C11 union's value to read `member` where it may be in the
/// cases `left` ([`ComposedSpelling::union_member`]), as [`union_member_of`]
/// reads it: the tag's member, or the field of the one case left through that
/// case's member of `as`. A field several cases share has no member path that
/// names it in each: the expression lowering reads it by a test of the tag over
/// the cases left ([`c11_shared_union_read`]), so `Err` here names that read.
pub(crate) fn c11_union_member(
    union: &ScopedUnion,
    left: &[&ScopedCase],
    member: &str,
) -> Result<String, String> {
    match union_member_of(union, left, member)? {
        UnionMember::Tag => Ok(C11_UNION_TAG.to_string()),
        UnionMember::Case(case) => Ok(format!("{}.{member}", c11_case_read(&case.variant.name))),
        UnionMember::Shared => Err(format!(
            "`{member}` of `union:{}`, which the cases {} share, is read on C11 by a test of the \
             tag over those cases, which no member path after the union's value spells \
             (docs/adr/0092, section 6.1)",
            union.name,
            left.iter()
                .map(|case| format!("`{}`", case.variant.name))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// The read of `member` of a C11 union where it may be in the cases `left`
/// and is a field those cases share ([`UnionMember::Shared`]): one member path
/// after the union's value per case left, each with the constant of the tag's
/// enum that names that case, which `constant` answers for a variant's
/// declared name. C has no expression that takes a case apart, so the
/// expression lowering reads it by a conditional over the tag whose last arm
/// is the last case, the narrowing having proved the value is in one of them.
/// `None` for any other read, which [`c11_union_member`] spells.
pub(crate) fn c11_shared_union_read(
    union: &ScopedUnion,
    left: &[&ScopedCase],
    member: &str,
    constant: &dyn Fn(&str) -> Option<String>,
) -> Result<Option<Vec<(String, String)>>, String> {
    if union_member_of(union, left, member)? != UnionMember::Shared {
        return Ok(None);
    }
    left.iter()
        .map(|case| {
            let variant = case.variant.name.as_str();
            let tag = constant(variant).ok_or_else(|| {
                format!(
                    "`union:{}`: the statechart's enum of its tag declares no variant \
                     `{variant}`",
                    union.name
                )
            })?;
            Ok((tag, format!("{}.{member}", c11_case_read(variant))))
        })
        .collect::<Result<Vec<_>, String>>()
        .map(Some)
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
/// reads (a record or a union the statechart does not hold).
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
        // A field of a case stands at the union's own place (docs/adr/0092,
        // section 2.1), so the longest path below a union is the longest below
        // any case's record, or the tag's, where a refusal of the tag stands.
        ValueType::Union { alias } => {
            let held = records.union(within, alias)?;
            let mut longest = 1 + held.union.tag.len() as u64;
            for case in &held.union.cases {
                if let Some(record) = &case.record {
                    let below = c11_path_len(
                        &ValueType::Record {
                            alias: record.clone(),
                        },
                        Some(&held.key),
                        records,
                    )?;
                    longest = longest.max(below);
                }
            }
            longest
        }
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
    // Into the buffer of the bound, its terminator after it; a text past it is
    // refused with that bound.
    fn read_text(&self, max: u32, value: &str) -> String {
        let ty = self.text_type(max);
        self.reader_of(stem(&ty), &ty, value, || {
            refused_at_bound(
                &format!("sce_payload_value_text(sce_value, sce_out->data, {max}u)"),
                max,
            )
        })
    }
    fn read_bytes(&self, max: u32, value: &str) -> String {
        let ty = self.bytes_type(max);
        self.reader_of(stem(&ty), &ty, value, || {
            refused_at_bound(
                &format!(
                    "sce_payload_value_bytes(sce_value, sce_out->data, {max}u, &sce_out->len)"
                ),
                max,
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
    // 15.5), with the count it broke; each element is read into the next slot,
    // at its index's path.
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
                 if (sce_refusal != NULL) {{\n        \
                 *{C11_BOUND} = (sce_refusal == SCE_PAYLOAD_REFUSE_SHORT_COUNT) ? {least}u : \
                 {max}u;\n        return sce_refusal;\n    }}\n    \
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
        unreachable!(
            "a C11 value is lifted into the optional value of the type of the place it is \
             copied into, which the walk names (`copy_present_into`)"
        )
    }
    // The optional value of the place's own element, made present by the
    // constructor its declaration holds (`sce/forge/optional.h`): a union put
    // whole into a variable that may be absent (docs/adr/0092, section 6.1 item
    // 5).
    fn copy_present_into(&self, value: &str, present: &str) -> String {
        let Some(name) = crate::forge::static_lowering::c11_optional_elem(present) else {
            unreachable!(
                "every C11 type a part is held in ends in `_t` or is a scalar: `{present}`"
            )
        };
        format!("sce_optional_{name}_some({value})")
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

    // A record that holds a composition carries its saved form in the
    // functions a record of entries' has (`<type>_saved_write`,
    // `<type>_saved_read`, `StaticTarget::record_def`), so a union's saved
    // form reads one spelling for either: an object of its fields keyed by
    // their ids in the schema's order, each written and read as its type says
    // ([`c11_saved_writer`], [`c11_saved_reader`]), a list held to its bound
    // and to the lower count its type states. Its members are written by a
    // function of their own, which a union's saved form writes beside its tag.
    fn saved_record_ext(
        &self,
        alias: &str,
        records: &dyn RecordEnv,
        enums: &BTreeMap<String, MachineEnum>,
    ) -> Result<Option<String>, String> {
        let held = records.record(None, alias)?;
        let ty = self.record_type(alias);
        let mut writes = String::new();
        let mut reads = String::new();
        for field in &held.schema.fields {
            let id = crate::filters::escape_c(field.id.clone());
            let refused =
                |why: String| format!("the field `{}` of `{}`: {why}", field.id, held.schema.name);
            // The fields of the record name their types in its own document's scope.
            let names = (records, enums, Some(held.alias));
            let write = c11_saved_writer(
                &field.sce_type,
                names,
                self,
                &format!("value->{}", field.id),
                1,
            )
            .map_err(refused)?;
            let read = c11_saved_reader(
                &field.sce_type,
                names,
                self,
                ("member", "member_what"),
                &format!("value.{}", field.id),
                1,
            )
            .map_err(refused)?;
            writes.push_str(&format!("    sce_jw_key(w, \"{id}\");\n{write}"));
            reads.push_str(&format!(
                "    if (!sce_saved_field(v, what, \"{id}\", &member, err)) {{\n        return \
                 false;\n    }}\n    sce_saved_path_field(member_what, sizeof(member_what), what, \
                 \"{id}\");\n{read}"
            ));
        }
        Ok(Some(format!(
            "/* The members of `value` as a saved state writes them, each keyed by its id, into \
             the\n   object being written. */\n\
             static inline void {ty}_saved_write_members(sce_jw_t *w, const {ty} *value) {{\n\
             {writes}}}\n\n\
             /* `value` as a saved state writes it: an object of the schema's fields keyed by \
             their ids. */\n\
             static inline void {ty}_saved_write(sce_jw_t *w, const {ty} *value) {{\n    \
             sce_jw_begin_object(w);\n    {ty}_saved_write_members(w, value);\n    \
             sce_jw_end_object(w);\n}}\n\n\
             /* The value a saved state holds, refused unless it is one, each list held to its \
             bound and\n   to the lower count its type states. `out` may be NULL, which judges \
             the value and keeps\n   nothing. */\n\
             static inline bool {ty}_saved_read(sce_jv_t v, const char *what, unsigned at, {ty} \
             *out,\n    sce_saved_error_t *err) {{\n    {ty} value;\n    char \
             member_what[SCE_SAVED_AT_LEN];\n    sce_jv_t member;\n    (void)at;\n    \
             memset(&value, 0, sizeof(value));\n    if (sce_jv_kind(v) != SCE_JSON_OBJECT) {{\n        \
             return sce_saved_refuse(err, \"'%s' is not an object\", what);\n    }}\n{reads}    \
             if (out != NULL) {{\n        *out = value;\n    }}\n    return true;\n}}"
        )))
    }
    // The union's saved form is one object, its tag beside the fields of its
    // case (docs/adr/0092, section 6.1 item 7): the tag written first by the
    // runtime's `sce_saved_write_union_tag`, as the declared name the union's
    // own function answers, and the case's record's members after it; read by
    // its tag first (`sce_saved_read_union_tag`), which names the record the
    // rest of the object is read as, through the constructor of the case.
    fn saved_union_ext(
        &self,
        def: &UnionDef,
        key: &str,
        records: &dyn RecordEnv,
    ) -> Result<Option<String>, String> {
        let tag = crate::filters::escape_c(records.union_of_key(key)?.union.tag.clone());
        let ty = &def.ty;
        let stem = stem(ty);
        let mut writes = String::new();
        let mut names = String::new();
        let mut reads = String::new();
        for (index, case) in def.cases.iter().enumerate() {
            names.push_str(&format!(
                "        \"{}\",\n",
                crate::filters::escape_c(case.variant.clone())
            ));
            let ident = &case.ident;
            let otherwise = if index == 0 { "" } else { "} else " };
            match &case.record {
                Some(record) => {
                    writes.push_str(&format!(
                        "    {otherwise}if (value->{C11_UNION_TAG} == {}) {{\n        \
                         {record}_saved_write_members(w, &value->{C11_UNION_CASES}.{ident});\n",
                        case.tag_ident
                    ));
                    reads.push_str(&format!(
                        "    {otherwise}if (index == {index}u) {{\n        {record} record;\n        \
                         memset(&record, 0, sizeof(record));\n        if (!{record}_saved_read(v, \
                         what, at, &record, err)) {{\n            return false;\n        }}\n        \
                         value = {stem}_case_{ident}(record);\n"
                    ));
                }
                None => {
                    writes.push_str(&format!(
                        "    {otherwise}if (value->{C11_UNION_TAG} == {}) {{\n        /* Its tag \
                         alone. */\n",
                        case.tag_ident
                    ));
                    reads.push_str(&format!(
                        "    {otherwise}if (index == {index}u) {{\n        value = \
                         {stem}_case_{ident}();\n"
                    ));
                }
            }
        }
        let name = crate::filters::escape_c(def.name.clone());
        Ok(Some(format!(
            "/* `value` as a saved state writes it: one object, its tag first beside the fields \
             of its case. */\n\
             static inline void {ty}_saved_write(sce_jw_t *w, const {ty} *value) {{\n    \
             const char *const declared = {stem}_variant(value);\n    \
             sce_jw_begin_object(w);\n    sce_saved_write_union_tag(w, \"{tag}\", declared != \
             NULL ? declared : \"\");\n{writes}    }}\n    sce_jw_end_object(w);\n}}\n\n\
             /* The value a saved state holds, read by its tag first, refused unless it is one of \
             the\n   cases of {name}. `out` may be NULL, which judges the value and keeps nothing. \
             */\n\
             static inline bool {ty}_saved_read(sce_jv_t v, const char *what, unsigned at, {ty} \
             *out,\n    sce_saved_error_t *err) {{\n    static const char *const names[] = \
             {{\n{names}    }};\n    size_t index = 0u;\n    {ty} value;\n    \
             memset(&value, 0, sizeof(value));\n    if (!sce_saved_read_union_tag(v, what, \
             \"{tag}\", \"{name}\", names, sizeof(names) / sizeof(names[0]), &index, err)) {{\n        \
             return false;\n    }}\n{reads}    }}\n    (void)at;\n    if (out != NULL) {{\n        \
             *out = value;\n    }}\n    return true;\n}}"
        )))
    }

    fn take_declared(&self) -> Vec<String> {
        self.take_declarations()
    }
    // C has no closure: a reader of a part is a function of the machine, each
    // guarded by its name, after the types it reads into.
    fn take_functions(&self) -> Vec<String> {
        let mut defs = self.take_declarations();
        defs.extend(self.take_readers());
        defs
    }

    // The machine's own struct, beside its records (docs/adr/0092, section 7,
    // the C11 row): named by the union's document, so one union is one type of
    // the machine whichever alias names it.
    fn union_type(&self, name: &str) -> String {
        format!(
            "{}_union_{}_t",
            self.machine,
            crate::filters::to_snake_case(name.to_string())
        )
    }
    // The variant in snake case, the case's member of the union's `as`.
    fn union_case_ident(&self, variant: &str) -> String {
        c11_case_ident(variant)
    }
    // A member of `as` is named as the variant is: one C reserves, or one the
    // preprocessor expands, has no member of that name to be.
    fn union_case_refused(&self, variant: &str) -> Option<String> {
        let ident = c11_case_ident(variant);
        crate::reader_names::is_reserved_word(crate::generator::Language::C11, &ident).then(|| {
            format!(
                "the case `{variant}` is held in the C11 member `{ident}`, a word C reserves; \
                 rename the variant"
            )
        })
    }
    // The tag is the one member every union of a C11 unit and of a C11 machine
    // holds it in, whatever key it is written under.
    fn union_tag_read(&self, _tag: &str) -> Option<String> {
        Some(C11_UNION_TAG.to_string())
    }
    fn union_case_read(&self, variant: &str) -> String {
        c11_case_read(variant)
    }
    fn union_shared_read(&self, field: &str) -> String {
        c11_shared_read(field)
    }
    // A struct of the tag and a C `union` of one member per case that holds a
    // record (section 7, the C11 row): its storage is the tag and the largest
    // case's record, the compiler's `sizeof` (section 5.1). A value is built
    // only by the constructor of its case, which zeroes the rest of the value
    // first, so two equal values are the same bytes (section 7 item 3). A read
    // the narrowing proves is the case's member itself; beside the type stand
    // the declared name of the case a value is in, which its writers name it
    // by, and the place of each field several cases share, which a write goes
    // through ([`Self::union_member_place`]) and whose other cases end the
    // process, the one place they are written.
    fn union_def(&self, def: &UnionDef) -> String {
        let ty = &def.ty;
        let stem = stem(ty);
        let mut members = String::new();
        let mut declared = String::new();
        let mut built = String::new();
        for case in &def.cases {
            let (ident, variant, tag) = (&case.ident, &case.variant, &case.tag_ident);
            declared.push_str(&format!(
                "    if (value->{C11_UNION_TAG} == {tag}) {{\n        return \"{}\";\n    }}\n",
                crate::filters::escape_c(variant.clone())
            ));
            match &case.record {
                Some(record) => {
                    members.push_str(&format!("        {record} {ident};\n"));
                    built.push_str(&format!(
                        "\n\n/* The case `{variant}`, holding `record`. */\n\
                         static inline {ty} {stem}_case_{ident}({record} record) {{\n    {ty} \
                         value;\n    memset(&value, 0, sizeof(value));\n    \
                         value.{C11_UNION_TAG} = {tag};\n    \
                         value.{C11_UNION_CASES}.{ident} = record;\n    return value;\n}}"
                    ));
                }
                None => built.push_str(&format!(
                    "\n\n/* The case `{variant}`, its tag alone. */\n\
                     static inline {ty} {stem}_case_{ident}(void) {{\n    {ty} value;\n    \
                     memset(&value, 0, sizeof(value));\n    value.{C11_UNION_TAG} = {tag};\n    \
                     return value;\n}}"
                )),
            }
        }
        let mut places = String::new();
        for field in &def.shared {
            let arms: String = field
                .cases
                .iter()
                .filter_map(|ident| def.cases.iter().find(|case| case.ident == *ident))
                .map(|case| {
                    format!(
                        "    if (value->{C11_UNION_TAG} == {}) {{\n        return \
                         &value->{C11_UNION_CASES}.{}.{};\n    }}\n",
                        case.tag_ident, case.ident, field.field
                    )
                })
                .collect();
            let (id, fty) = (&field.field, &field.ty);
            let place = self.union_place_of(&field.accessor);
            places.push_str(&format!(
                "\n\n/* The field `{id}` the cases that declare it share, written where a test of \
                 the tag\n   proved the value is in one of them. */\n\
                 static inline {fty} *{stem}_{place}({ty} *value) {{\n{arms}    abort();\n}}"
            ));
        }
        format!(
            "/* SCE Accepted Subset §2.15: a `union:{}` value (docs/adr/0092): exactly one case,\n   \
             told apart by its tag, holding the record of its case in `{C11_UNION_CASES}`, \
             built only whole\n   by the constructor of its case (section 7, the C11 row, and \
             its item 3). */\n\
             typedef struct {{\n    {} {C11_UNION_TAG};\n    union {{\n{members}    }} \
             {C11_UNION_CASES};\n}} {ty};\n\n\
             /* The name the tag's document declares for the case `value` is in, which the \
             value is\n   written by (section 2.1); NULL for a tag that names no case, which \
             no constructor builds. */\n\
             static inline const char *{stem}_variant(const {ty} *value) {{\n{declared}    \
             return NULL;\n}}{built}{places}",
            def.name, def.tag_ty
        )
    }
    // The constructor of the case, handed the record or nothing.
    fn union_value(&self, ty: &str, _tag: &str, case: &str, record: Option<&str>) -> String {
        format!("{}_case_{case}({})", stem(ty), record.unwrap_or_default())
    }
    // The tag first, against the names its enum declares, by the runtime's
    // `sce_payload_union_open`, which refuses a value that is no object before
    // it and says whether a refusal stands at the tag, whose key the path then
    // names; then the record of the case the tag names, read from the same
    // object by the reader of the record (docs/adr/0092, section 4), and the
    // value built by the case's constructor. The tag's reader answers only a
    // case the union declares, so the last `return` is not reached.
    fn read_union(&self, value: &str, read: &UnionRead<'_>) -> String {
        let ty = &read.ty;
        let names: String = read
            .arms
            .iter()
            .map(|arm| format!("\"{}\", ", crate::filters::escape_c(arm.variant.clone())))
            .collect();
        let tag = crate::filters::escape_c(read.tag.to_string());
        let arms: String = read
            .arms
            .iter()
            .enumerate()
            .map(|(index, arm)| {
                let built = |record: Option<&str>| self.union_value(ty, read.tag, &arm.case_ident, record);
                match &arm.record {
                    Some(record) => format!(
                        "    if (sce_index == {index}u) {{\n        \
                         __typeof__(sce_out->{C11_UNION_CASES}.{case}) sce_case;\n        \
                         memset(&sce_case, 0, sizeof(sce_case));\n        {{\n            \
                         const char *const {C11_READ_BINDING} = sce_value;\n            \
                         void *const {C11_INTO} = &sce_case;\n            \
                         sce_refusal = {record};\n        }}\n        \
                         if (sce_refusal != NULL) {{\n            return sce_refusal;\n        }}\n        \
                         *sce_out = {};\n        return NULL;\n    }}\n",
                        built(Some("sce_case")),
                        case = arm.case_ident,
                    ),
                    None => format!(
                        "    if (sce_index == {index}u) {{\n        *sce_out = {};\n        \
                         return NULL;\n    }}\n",
                        built(None)
                    ),
                }
            })
            .collect();
        self.reader_of(stem(ty), ty, value, || {
            format!(
                "    static const char *const sce_names[] = {{ {names}}};\n    \
                 size_t sce_index = 0u;\n    bool sce_at_tag = false;\n    \
                 const char *sce_refusal = sce_payload_union_open(sce_value, \"{tag}\", sce_names, \
                 sizeof(sce_names) / sizeof(sce_names[0]), &sce_index, &sce_at_tag);\n    \
                 if (sce_refusal != NULL) {{\n        if (sce_at_tag) {{\n            \
                 (void)sce_payload_path_join({C11_PATH}, {C11_PATH_CAP}, {C11_PATH}, \"{tag}\");\n        \
                 }}\n        return sce_refusal;\n    }}\n{arms}    \
                 return SCE_PAYLOAD_REFUSE_UNKNOWN_VARIANT;\n"
            )
        })
    }
    // One object, the tag among the fields of the case, its keys ascending by
    // their bytes (docs/adr/0092, section 2.1): the case is told by the
    // declared name its union's own function answers, which the tag is written
    // as, and its record is bound in its arm as a record's writer reads it. A
    // value whose tag names no case, which no constructor builds but a host's
    // hand might, refuses the object whole (section 5.4).
    fn write_union(&self, held: &Held, ty: &str, tag: &str, arms: &[UnionWriteArm]) -> String {
        let value = value_of(held);
        let tag_written = Self::put("sce_forge_wire_string(sce_variant)");
        let arms: String = arms
            .iter()
            .map(|arm| {
                let variant = crate::filters::escape_c(arm.variant.clone());
                let mut members = arm.fields.clone().unwrap_or_default();
                members.push((tag.to_string(), tag_written.clone()));
                let written = self.write_record(&members);
                match &arm.fields {
                    Some(_) => format!(
                        " else if (strcmp(sce_variant, \"{variant}\") == 0) {{\n    \
                         __typeof__(sce_union->{C11_UNION_CASES}.{case}) const *const sce_next = \
                         &sce_union->{C11_UNION_CASES}.{case};\n    {{\n        \
                         __typeof__(sce_next) {C11_WRITE_BINDING} = sce_next;\n{written}    }}\n}}",
                        case = arm.case_ident
                    ),
                    None => format!(
                        " else if (strcmp(sce_variant, \"{variant}\") == 0) {{\n{written}}}"
                    ),
                }
            })
            .collect();
        format!(
            "{{\n    __typeof__({value}) const *const sce_union = &{value};\n    \
             const char *const sce_variant = {}_variant(sce_union);\n    \
             if (sce_variant == NULL) {{\n        sce_forge_wire_refuse({C11_WIRE});\n    }}\
             {arms} else {{\n        sce_forge_wire_refuse({C11_WIRE});\n    }}\n}}\n",
            stem(ty)
        )
    }
    fn union_member(
        &self,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        c11_union_member(union, left, member)
    }
    fn union_place_of(&self, read: &str) -> String {
        format!("{read}_place")
    }
    // A field is written in place: the field of the one case left is its
    // member, and a field the cases left share is written through the place
    // the union's own function answers, which takes the holder's address.
    fn union_member_place(
        &self,
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
        Ok(match union_member_of(union, left, member)? {
            UnionMember::Case(case) => {
                format!("{holder}.{}.{member}", c11_case_read(&case.variant.name))
            }
            UnionMember::Shared => format!(
                "(*{}_{}(&{holder}))",
                stem(&self.union_type(&union.name)),
                self.union_place_of(&c11_shared_read(member))
            ),
            UnionMember::Tag => unreachable!("the tag is refused above"),
        })
    }
}

/// The names a C11 saved form of a value reads: the records and unions it
/// holds, the machine's enums, and the scope its type is written in.
type C11SavedNames<'n> = (
    &'n dyn RecordEnv,
    &'n BTreeMap<String, MachineEnum>,
    Option<&'n str>,
);

/// The C statements that write `value`, an lvalue of a value of `ty`, as a saved
/// state does (`sce/saved_values.h`) into the writer `w`, where its key or its
/// element stands: a number and a truth value by the runtime's writer of its
/// type, an enum, a record and a union by their own `<type>_saved_write`, a text
/// and a byte string from the buffer that holds them, a value that may be
/// absent as `null` or its present value, and a list element by element. `depth`
/// names each loop's index, so none hides its holder's. `Err` for a byte string
/// inside a list, which a saved state would write as an array where it writes a
/// byte string as its text.
fn c11_saved_writer(
    ty: &ValueType,
    names: C11SavedNames<'_>,
    spelling: &C11Composed<'_>,
    value: &str,
    depth: usize,
) -> Result<String, String> {
    let (records, enums, within) = names;
    let pad = "    ".repeat(depth);
    Ok(match ty {
        ValueType::Scalar(SceType::Enum(_)) | ValueType::Record { .. } | ValueType::Union { .. } => {
            let held = machine_type(ty, within, records, enums, spelling)?;
            if matches!(ty, ValueType::Scalar(_)) {
                format!("{pad}{held}_saved_write(w, {value});\n")
            } else {
                format!("{pad}{held}_saved_write(w, &({value}));\n")
            }
        }
        ValueType::Scalar(scalar) => {
            format!("{pad}sce_saved_write_{}(w, {value});\n", scalar.as_attr())
        }
        ValueType::Text { .. } => format!("{pad}sce_saved_write_string(w, ({value}).data);\n"),
        ValueType::Bytes { .. } => {
            format!("{pad}sce_saved_write_bytes(w, ({value}).data, ({value}).len);\n")
        }
        ValueType::Optional { elem } => format!(
            "{pad}if (({value}).present) {{\n{}{pad}}} else {{\n{pad}    sce_jw_null(w);\n{pad}}}\n",
            c11_saved_writer(elem, names, spelling, &format!("({value}).value"), depth + 1)?
        ),
        ValueType::List { elem, .. } => {
            if matches!(**elem, ValueType::Bytes { .. }) {
                return Err(
                    "a byte string inside a list, which a saved state has no text for".to_string(),
                );
            }
            let at = format!("at_{depth}");
            format!(
                "{pad}sce_jw_begin_array(w);\n{pad}for (size_t {at} = 0u; {at} < ({value}).len; \
                 ++{at}) {{\n{pad}    sce_jw_element(w);\n{}{pad}}}\n{pad}sce_jw_end_array(w);\n",
                c11_saved_writer(
                    elem,
                    names,
                    spelling,
                    &format!("({value}).data[{at}]"),
                    depth + 1
                )?
            )
        }
    })
}

/// The C statements that read the saved value `member` (a `sce_jv_t`), which
/// `what` (a `const char *`) names in a refusal, into `dest`, an lvalue of a
/// value of `ty`, as [`c11_saved_writer`] writes it, each ending the reading
/// function with `false` at the first refusal: a text and a byte string into the
/// buffer of the bound the type states, a list to its bound and to the lower
/// count it states, an enum, a record and a union by their own
/// `<type>_saved_read`, and a value that may be absent read absent from `null`.
/// A record and a union are read at the revision `at` the reading function was
/// handed. `depth` names each loop's locals, so none hides its holder's.
fn c11_saved_reader(
    ty: &ValueType,
    names: C11SavedNames<'_>,
    spelling: &C11Composed<'_>,
    from: (&str, &str),
    dest: &str,
    depth: usize,
) -> Result<String, String> {
    let (records, enums, within) = names;
    let (member, what) = from;
    let pad = "    ".repeat(depth);
    let refused = |read: String| format!("{pad}if (!{read}) {{\n{pad}    return false;\n{pad}}}\n");
    Ok(match ty {
        ValueType::Scalar(SceType::Enum(_)) => refused(format!(
            "{}_saved_read({member}, {what}, &({dest}), err)",
            machine_type(ty, within, records, enums, spelling)?
        )),
        ValueType::Record { .. } | ValueType::Union { .. } => refused(format!(
            "{}_saved_read({member}, {what}, at, &({dest}), err)",
            machine_type(ty, within, records, enums, spelling)?
        )),
        ValueType::Scalar(scalar) => refused(format!(
            "sce_saved_read_{}({member}, {what}, &({dest}), err)",
            scalar.as_attr()
        )),
        ValueType::Text { max } => refused(format!(
            "sce_saved_read_string_bounded({member}, {what}, ({dest}).data, {}u, err)",
            max.max
        )),
        ValueType::Bytes { max } => refused(format!(
            "sce_saved_read_bytes({member}, {what}, ({dest}).data, {}u, &({dest}).len, err)",
            max.max
        )),
        ValueType::Optional { elem } => format!(
            "{pad}if (sce_jv_is_null({member})) {{\n{pad}    ({dest}).present = false;\n{pad}}} \
             else {{\n{pad}    ({dest}).present = true;\n{}{pad}}}\n",
            c11_saved_reader(
                elem,
                names,
                spelling,
                from,
                &format!("({dest}).value"),
                depth + 1
            )?
        ),
        ValueType::List { elem, max } => {
            let (count, it, item, at, at_what) = (
                format!("count_{depth}"),
                format!("it_{depth}"),
                format!("item_{depth}"),
                format!("at_{depth}"),
                format!("what_{depth}"),
            );
            format!(
                "{pad}{{\n{pad}    size_t {count} = 0u;\n{pad}    sce_jiter_t {it};\n{pad}    \
                 sce_jv_t {item};\n{pad}    char {at_what}[SCE_SAVED_AT_LEN];\n{pad}    if \
                 (!sce_saved_list_within({member}, {what}, {}u, {}u, &{count}, err)) {{\n{pad}        \
                 return false;\n{pad}    }}\n{pad}    (void){count};\n{pad}    \
                 (void)sce_jv_iter({member}, &{it});\n{pad}    ({dest}).len = 0u;\n{pad}    for \
                 (size_t {at} = 0u; sce_jiter_element(&{it}, &{item}); ++{at}) {{\n{pad}        \
                 sce_saved_path_index({at_what}, sizeof({at_what}), {what}, {at});\n{}{pad}        \
                 ({dest}).len = {at} + 1u;\n{pad}    }}\n{pad}}}\n",
                max.min,
                max.max,
                c11_saved_reader(
                    elem,
                    names,
                    spelling,
                    (item.as_str(), at_what.as_str()),
                    &format!("({dest}).data[{at}]"),
                    depth + 2
                )?
            )
        }
    })
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
    /// value sent (docs/adr/0086, sections 5 and 6), and a union held, read by
    /// its tag, lifted and saved (docs/adr/0092, section 7); no variable of a
    /// record that holds a composition yet.
    #[test]
    fn the_c11_row_lowers_every_composed_position_but_a_composed_record_variable() {
        use crate::forge::event_schema_check::ComposedLowering;
        assert_eq!(
            ComposedLowering::of(crate::generator::Language::C11),
            ComposedLowering {
                name: "c11",
                reads: true,
                copies: true,
                unions: true,
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
            "sce_composed_read_sce_static_list_uint8_2_list_3(v, sce_into, sce_path, sce_path_cap, \
             sce_bound)"
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
        // Each reader stands under a guard of its name, so a machine that reads one
        // type through two spellings defines its reader once, and writes the bound a
        // list breaks, past its bound or short of its lower count.
        assert!(
            readers[0].starts_with(
                "#ifndef SCE_COMPOSED_READ_SCE_STATIC_LIST_UINT8_2\n\
                 #define SCE_COMPOSED_READ_SCE_STATIC_LIST_UINT8_2\n"
            ) && readers[0].ends_with("#endif\n")
                && readers[0].contains("size_t sce_path_cap, uint32_t *sce_bound) {")
                && readers[0].contains(
                    "*sce_bound = (sce_refusal == SCE_PAYLOAD_REFUSE_SHORT_COUNT) ? 0u : 2u;"
                ),
            "{}",
            readers[0]
        );
        assert!(
            readers[1].contains("sce_payload_list_open_within(sce_value, 0u, 3u, &sce_cursor)")
                && readers[1].contains(
                    "sce_refusal = sce_composed_read_sce_static_list_uint8_2(sce_at, sce_into, \
                     sce_path, sce_path_cap, sce_bound);"
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
             sce_path_cap, sce_bound)"
        );
        let within = spelling.take_readers().join("\n");
        assert!(
            within.contains("sce_payload_list_open_within(sce_value, 1u, 2u, &sce_cursor)")
                && within.contains(
                    "*sce_bound = (sce_refusal == SCE_PAYLOAD_REFUSE_SHORT_COUNT) ? 1u : 2u;"
                ),
            "{within}"
        );
        // A text and a byte string into the buffer of their bound, held to it, the
        // bound written where it refuses.
        let _ = read("string[8]");
        let _ = read("bytes[4]");
        let readers = spelling.take_readers().join("\n");
        assert!(
            readers.contains(
                "    const char *const sce_refusal = sce_payload_value_text(sce_value, \
                 sce_out->data, 8u);\n    if (sce_refusal != NULL) {\n        *sce_bound = \
                 8u;\n    }\n    return sce_refusal;\n"
            ) && readers.contains(
                "const char *const sce_refusal = sce_payload_value_bytes(sce_value, \
                 sce_out->data, 4u, &sce_out->len);\n    if (sce_refusal != NULL) {\n        \
                 *sce_bound = 4u;"
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
        // A field of a case stands at the union's place: `/radius`, longer than
        // the tag's `/kind`.
        assert_eq!(
            c11_path_len(&ty("union:Shape"), None, &shapes()),
            Ok(7),
            "the longest place below a shape"
        );
        assert!(c11_path_len(&ty("union:Shape"), None, &held).is_err());
    }

    /// The case of the shape union named `name`.
    fn case<'s>(shapes: &'s Shapes, name: &str) -> &'s ScopedCase {
        shapes
            .union
            .cases
            .iter()
            .find(|case| case.variant.name == name)
            .expect("a case")
    }

    /// A union is the machine's own struct at any depth, named by its document
    /// (docs/adr/0092, section 7, the C11 row), its tag held in `tag` whatever
    /// key it is written under, and a value built whole by the constructor of
    /// its case.
    #[test]
    fn a_c11_machine_holds_a_union_in_its_own_struct() {
        let shapes = shapes();
        let kinds = kinds();
        let spelling = c11();
        assert_eq!(
            machine_type(&ty("union:Shape"), None, &shapes, &kinds, &spelling),
            Ok("m_union_shape_t".to_string())
        );
        assert_eq!(
            machine_type(
                &ty("list<optional<union:Shape>>[3]"),
                None,
                &shapes,
                &kinds,
                &spelling
            ),
            Ok("sce_optional_record_m_union_shape_list_3_t".to_string())
        );
        assert!(
            spelling
                .take_declarations()
                .join("\n")
                .contains("SCE_OPTIONAL_DECLARE_RECORD(record_m_union_shape, m_union_shape_t)"),
            "a union that may be absent is the runtime's struct over the union"
        );
        assert_eq!(spelling.union_tag_read("type"), Some("tag".to_string()));
        assert_eq!(
            spelling.union_value("m_union_shape_t", "type", "dot", None),
            "m_union_shape_case_dot()"
        );
        assert_eq!(
            spelling.union_value("m_union_shape_t", "type", "circle", Some("r")),
            "m_union_shape_case_circle(r)"
        );
        assert_eq!(
            spelling.copy_present_into("(p)", "m_union_shape_t"),
            "sce_optional_record_m_union_shape_some((p))"
        );
    }

    /// The declaration (section 7, the C11 row): a struct of the tag and a C
    /// `union` of a member per case that holds a record, an empty case none; the
    /// declared name of the case a value is in; a constructor per case that
    /// zeroes the value first (section 7 item 3); and the place of `radius`,
    /// which `circle` and `disc` share, ending the process in another case.
    #[test]
    fn a_c11_union_is_declared_with_its_tag_its_cases_and_its_constructors() {
        let spelling = c11();
        let def = union_definition(SHAPE_KEY, &shapes(), &kinds(), &spelling).expect("declared");
        assert_eq!(def.ty, "m_union_shape_t");
        assert_eq!(def.tag_method, "tag");
        assert_eq!(
            def.shared,
            vec![SharedFieldDef {
                field: "radius".to_string(),
                accessor: "proven_field_radius".to_string(),
                ty: "uint16_t".to_string(),
                cases: vec!["circle".to_string(), "disc".to_string()],
            }]
        );
        let text = spelling.union_def(&def);
        for fragment in [
            "typedef struct {\n    MKindEnum tag;\n    union {\n        m_record_radius_t circle;\n        \
             m_record_radius_t disc;\n    } as;\n} m_union_shape_t;\n",
            "static inline const char *m_union_shape_variant(const m_union_shape_t *value) {\n    \
             if (value->tag == Dot) {\n        return \"dot\";\n    }\n",
            "static inline m_union_shape_t m_union_shape_case_dot(void) {\n    m_union_shape_t \
             value;\n    memset(&value, 0, sizeof(value));\n    value.tag = Dot;\n    return \
             value;\n}",
            "static inline m_union_shape_t m_union_shape_case_circle(m_record_radius_t record) {\n    \
             m_union_shape_t value;\n    memset(&value, 0, sizeof(value));\n    value.tag = \
             Circle;\n    value.as.circle = record;\n    return value;\n}",
            "static inline uint16_t *m_union_shape_proven_field_radius_place(m_union_shape_t \
             *value) {\n    if (value->tag == Circle) {\n        return \
             &value->as.circle.radius;\n    }\n    if (value->tag == Disc) {\n        return \
             &value->as.disc.radius;\n    }\n    abort();\n}",
        ] {
            assert!(text.contains(fragment), "expected `{fragment}` in:\n{text}");
        }
        // An empty case holds no record and has no member.
        assert!(!text.contains(" dot;"), "{text}");
    }

    /// A case is a member named as its variant, so a variant C reserves the
    /// spelling of is refused by name before anything is declared.
    #[test]
    fn a_c11_case_spelled_as_a_word_c_reserves_is_refused() {
        let spelling = c11();
        assert_eq!(spelling.union_case_refused("circle"), None);
        let refused = spelling
            .union_case_refused("default")
            .expect("`default` is a word C reserves");
        assert!(refused.contains("`default`"), "{refused}");
    }

    /// A read of a union's member (docs/adr/0092, section 6.1): the tag by its
    /// member, a field of the one case left through that case's member of
    /// `as`, and a field two cases share by a test of the tag over them; a
    /// write goes through the member, or through the place the union's own
    /// function answers, and the tag is never written alone.
    #[test]
    fn a_c11_proven_read_and_write_are_spelled_by_the_cases_left() {
        let shapes = shapes();
        let union = &shapes.union;
        let spelling = c11();
        let all: Vec<&ScopedCase> = union.cases.iter().collect();
        let both = [case(&shapes, "circle"), case(&shapes, "disc")];
        let disc = [case(&shapes, "disc")];
        assert_eq!(c11_union_member(union, &all, "kind"), Ok("tag".into()));
        assert_eq!(
            spelling.union_member(union, &disc, "radius"),
            Ok("as.disc.radius".into())
        );
        let shared = c11_union_member(union, &both, "radius").expect_err("no one member path");
        assert!(shared.contains("test of the tag"), "{shared}");
        let constant = |variant: &str| Some(format!("K_{variant}"));
        assert_eq!(
            c11_shared_union_read(union, &both, "radius", &constant),
            Ok(Some(vec![
                ("K_circle".to_string(), "as.circle.radius".to_string()),
                ("K_disc".to_string(), "as.disc.radius".to_string()),
            ]))
        );
        assert_eq!(
            c11_shared_union_read(union, &disc, "radius", &constant),
            Ok(None)
        );
        assert!(c11_union_member(union, &[case(&shapes, "dot")], "radius").is_err());
        assert_eq!(
            spelling.union_member_place("v_s", union, &disc, "radius"),
            Ok("v_s.as.disc.radius".into())
        );
        assert_eq!(
            spelling.union_member_place("v_s", union, &both, "radius"),
            Ok("(*m_union_shape_proven_field_radius_place(&v_s))".into())
        );
        let tag = spelling
            .union_member_place("v_s", union, &all, "kind")
            .expect_err("the tag is never written alone");
        assert!(tag.contains("never written alone"), "{tag}");
    }

    /// A union is read by a function of the machine (docs/adr/0092, section
    /// 4): its tag first, against the names its enum declares, a refusal at the
    /// tag naming its key in the path; then the record of the case it names
    /// from the same object, by the reader of the record, defined before it.
    #[test]
    fn a_c11_union_is_read_by_its_tag_first() {
        let shapes = shapes();
        let spelling = c11();
        assert_eq!(
            reader(&ty("union:Shape"), None, &shapes, &kinds(), &spelling, "v"),
            Ok(
                "sce_composed_read_m_union_shape(v, sce_into, sce_path, sce_path_cap, sce_bound)"
                    .to_string()
            )
        );
        let readers = spelling.take_readers().join("\n");
        for fragment in [
            "static const char *const sce_names[] = { \"dot\", \"circle\", \"disc\", };",
            "const char *sce_refusal = sce_payload_union_open(sce_value, \"kind\", sce_names, \
             sizeof(sce_names) / sizeof(sce_names[0]), &sce_index, &sce_at_tag);",
            "        if (sce_at_tag) {\n            (void)sce_payload_path_join(sce_path, \
             sce_path_cap, sce_path, \"kind\");\n        }\n",
            "    if (sce_index == 0u) {\n        *sce_out = m_union_shape_case_dot();\n        \
             return NULL;\n    }\n",
            "sce_refusal = sce_composed_read_m_record_radius(sce_at, sce_into, sce_path, \
             sce_path_cap, sce_bound);",
            "*sce_out = m_union_shape_case_disc(sce_case);",
        ] {
            assert!(
                readers.contains(fragment),
                "expected `{fragment}` in:\n{readers}"
            );
        }
        let record = readers
            .find("sce_composed_read_m_record_radius(const char *sce_value")
            .expect("the record's reader");
        let union = readers
            .find("sce_composed_read_m_union_shape(const char *sce_value")
            .expect("the union's reader");
        assert!(record < union, "{readers}");
    }

    /// A union is written as one object, its tag among its case's fields, its
    /// keys ascending, the case told by the declared name its union's function
    /// answers, a value in no case refusing the object.
    #[test]
    fn a_c11_union_is_written_as_its_tag_beside_its_case() {
        let shapes = shapes();
        let spelling = c11();
        let written = writer(
            &ty("union:Shape"),
            None,
            &shapes,
            &spelling,
            &Held::place("p"),
        )
        .expect("a writer");
        for fragment in [
            "__typeof__((p)) const *const sce_union = &(p);",
            "const char *const sce_variant = m_union_shape_variant(sce_union);",
            "if (sce_variant == NULL) {\n        sce_forge_wire_refuse(sce_wire);\n    }",
            " else if (strcmp(sce_variant, \"disc\") == 0) {\n    __typeof__(sce_union->as.disc) \
             const *const sce_next = &sce_union->as.disc;",
            "sce_forge_wire_put_value(sce_wire, sce_forge_wire_string(sce_variant));",
            "sce_forge_wire_uint((uint64_t)((*sce_item).radius))",
        ] {
            assert!(
                written.contains(fragment),
                "expected `{fragment}` in:\n{written}"
            );
        }
        // The keys ascend: `kind` before `radius` in a case that holds both.
        let disc = written.find("\"disc\") == 0").expect("the disc arm");
        let kind = written[disc..]
            .find("\"\\\"kind\\\":\"")
            .expect("the tag's key");
        let radius = written[disc..]
            .find("\"\\\"radius\\\":\"")
            .expect("the radius key");
        assert!(kind < radius, "{written}");
    }

    /// The saved form (docs/adr/0092, section 6.1 item 7): one object, the tag
    /// written first beside the members of its case's record, read back by its
    /// tag first and built by the constructor of the case.
    #[test]
    fn a_c11_saved_union_is_its_tag_beside_its_case_and_is_read_back_by_its_tag() {
        let shapes = shapes();
        let spelling = c11();
        let def = union_definition(SHAPE_KEY, &shapes, &kinds(), &spelling).expect("declared");
        let saved = spelling
            .saved_union_ext(&def, SHAPE_KEY, &shapes)
            .expect("a saved form")
            .expect("a C11 union carries its saved form");
        for fragment in [
            "static inline void m_union_shape_t_saved_write(sce_jw_t *w, const m_union_shape_t \
             *value) {",
            "sce_saved_write_union_tag(w, \"kind\", declared != NULL ? declared : \"\");",
            "} else if (value->tag == Circle) {\n        \
             m_record_radius_t_saved_write_members(w, &value->as.circle);\n",
            "if (!sce_saved_read_union_tag(v, what, \"kind\", \"shape\", names, sizeof(names) / \
             sizeof(names[0]), &index, err)) {",
            "    if (index == 0u) {\n        value = m_union_shape_case_dot();\n",
            "if (!m_record_radius_t_saved_read(v, what, at, &record, err)) {",
            "value = m_union_shape_case_disc(record);",
        ] {
            assert!(
                saved.contains(fragment),
                "expected `{fragment}` in:\n{saved}"
            );
        }
    }

    /// A record that holds a composition carries its saved form (section 6.1
    /// item 7), its members written by a function of their own a union writes
    /// beside its tag, each list held to its bound and its lower count as it is
    /// read back, element by element.
    #[test]
    fn a_c11_composed_record_carries_its_saved_form() {
        let held = held();
        let spelling = c11();
        let saved = spelling
            .saved_record_ext("Batch", &held, &c11_enums())
            .expect("a saved form")
            .expect("a C11 composed record carries its saved form");
        for fragment in [
            "static inline void m_record_batch_t_saved_write_members(sce_jw_t *w, const \
             m_record_batch_t *value) {\n    sce_jw_key(w, \"items\");\n    \
             sce_jw_begin_array(w);\n    for (size_t at_1 = 0u; at_1 < (value->items).len; \
             ++at_1) {\n        sce_jw_element(w);\n        m_record_item_t_saved_write(w, \
             &((value->items).data[at_1]));\n    }\n    sce_jw_end_array(w);\n",
            "    m_record_batch_t_saved_write_members(w, value);\n",
            "if (!sce_saved_list_within(member, member_what, 0u, 3u, &count_1, err)) {",
            "            if (!m_record_item_t_saved_read(item_1, what_1, at, \
             &((value.items).data[at_1]), err)) {",
            "            (value.items).len = at_1 + 1u;\n",
        ] {
            assert!(
                saved.contains(fragment),
                "expected `{fragment}` in:\n{saved}"
            );
        }
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
