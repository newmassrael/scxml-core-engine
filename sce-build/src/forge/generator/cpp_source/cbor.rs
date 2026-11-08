// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ printer of a CBOR codec's plan (docs/adr/0081 step 4, SCE_FORGE.md
//! §4.6.1).
//!
//! A CBOR map derives nothing a value says: the map head counts the entries
//! present, which is the encoding's and no element. So every entry is
//! presented, as the C++ interface presents a positional codec's elements: one
//! accessor for each, the value itself for a required entry and a
//! `std::optional` for one that may be absent, read through the `source`
//! customization points and checked by `is_source`. A text or a byte string is
//! presented within its bound (`Text`, `Bytes`; docs/adr/0077 §3), so the
//! encode over any source refuses nothing but where an enum's carrier is
//! signed, which is not printed yet. The encoding rule is the one the Rust
//! printer writes (`rust_source::cbor`): the map head, then each entry present
//! in ascending key order, every head in its shortest form (RFC 8949 §4.2.1).

use super::super::*;
use super::source_readers;
use crate::forge::encode_plan::{CborItem, CborPlan, CborValue};
use crate::forge::model::CborEntry;

/// Every form of the CBOR codec `plan` the C++ printer does not print yet: a
/// list of records, which the C++ backend lowers nowhere yet
/// (`cbor_codec::lowers_record_list`), and an encode that can refuse, which an
/// enum of a signed carrier or a list's lower count makes.
pub(in super::super) fn unlowered_forms(plan: &CborPlan<'_>) -> Vec<String> {
    let mut forms = Vec::new();
    if plan
        .entries
        .iter()
        .any(|item| matches!(item.value, CborValue::RecordList { .. }))
    {
        forms.push("a list of records".to_string());
    }
    if !plan.totality.is_total() {
        forms.push("an encode that can refuse".to_string());
    }
    forms
}

/// The type a present value of the leaf entry `e` is. Every entry the printer
/// sees is a leaf: a map that holds a list of records is not printed
/// (`unlowered_forms`).
fn leaf(e: &CborEntry) -> &SceType {
    e.leaf_type()
        .expect("a printed map holds no list of records")
}

/// The statement for a list of records, which no printed map holds.
fn no_record_list() -> ! {
    unreachable!("a printed map holds no list of records (`unlowered_forms`)")
}

/// One entry as the printed value presents and holds it.
struct Entry<'p> {
    item: &'p CborItem<'p>,
    /// Its accessor's name.
    name: String,
    /// What the value presents it as where it is there: a text or a byte
    /// string as its bound's view, anything else as its type.
    view: String,
    /// What the value holds it as where it is there: a text or a byte string
    /// in its bound's owning list.
    held: String,
    /// What the decode reads it as before it is held to its bound.
    read: String,
    /// The initializer of what the value holds, where a value made by
    /// default does not hold its type's zero: an enum starts at its first
    /// declared variant.
    init: String,
}

impl Entry<'_> {
    fn required(&self) -> bool {
        self.item.required
    }

    /// Whether it is a text or a byte string, presented as its bound's view.
    fn bounded(&self) -> bool {
        matches!(self.item.value, CborValue::Text | CborValue::Bytes { .. })
    }

    /// What a source presents it as: its view, or an optional of it.
    fn presented(&self) -> String {
        if self.required() {
            self.view.clone()
        } else {
            format!("std::optional<{}>", self.view)
        }
    }

    fn held_member(&self) -> String {
        if self.required() {
            self.held.clone()
        } else {
            format!("std::optional<{}>", self.held)
        }
    }
}

/// The bound a text or a byte string of `e` is held to, as `Max` or `Max, Min`.
fn bound_args(e: &CborEntry, exact: bool) -> String {
    match (exact, e.length, e.max_size) {
        (true, Some(length), _) => format!("{length}, {length}"),
        (_, _, Some(max)) => max.to_string(),
        _ => "::SCE::Forge::kUnbounded".to_string(),
    }
}

fn entry<'p>(l: &LangCtx, item: &'p CborItem<'p>) -> Entry<'p> {
    let e = item.entry;
    let ty = l.type_name(leaf(e)).into_owned();
    let (view, held) = match item.value {
        CborValue::Text => {
            let bound = bound_args(e, false);
            (
                format!("::SCE::Forge::Text<{bound}>"),
                format!("::SCE::Forge::Bounded<std::string, {bound}>"),
            )
        }
        CborValue::Bytes { exact } => {
            let bound = bound_args(e, exact);
            (
                format!("::SCE::Forge::Bytes<{bound}>"),
                format!("::SCE::Forge::Bounded<std::vector<std::uint8_t>, {bound}>"),
            )
        }
        _ => (ty.clone(), ty.clone()),
    };
    let init = match leaf(e) {
        SceType::Enum(r) if item.required => l.enum_default_expr(&r.alias),
        _ => String::new(),
    };
    Entry {
        item,
        name: l.codec_field_id(&e.id),
        view,
        held,
        read: ty,
        init,
    }
}

/// The statement that writes `v`, the value of `entry`, after its key.
fn write_value(l: &LangCtx, entry: &Entry<'_>) -> String {
    match entry.item.value {
        CborValue::Bool => "if (auto _e = cbor::write_bool(w, v); _e) return _e;".to_string(),
        CborValue::Text => {
            "if (auto _e = cbor::write_text(w, v.inner()); _e) return _e;".to_string()
        }
        CborValue::Bytes { .. } => {
            "if (auto _e = cbor::write_bytes(w, v.inner()); _e) return _e;".to_string()
        }
        // An unsigned carrier's every value is an unsigned integer; a signed
        // one is not printed (`unlowered_forms`).
        CborValue::Enum { .. } => format!(
            "if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>({})); _e) return _e;",
            l.codec_carrier_expr(leaf(entry.item.entry), "v")
        ),
        CborValue::Uint { .. } => {
            "if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>(v)); _e) return _e;"
                .to_string()
        }
        CborValue::RecordList { .. } => no_record_list(),
    }
}

/// The statements that read the value of `entry` into its local, which the
/// caller has checked is still empty; a refusal returns `std::nullopt` from
/// the decode, this backend's decode convention.
fn read_value(l: &LangCtx, imports: &[ImportContext], entry: &Entry<'_>) -> String {
    let e = entry.item.entry;
    let limit =
        |max: Option<u32>| max.map_or_else(|| "std::nullopt".to_string(), |m| m.to_string());
    let read = match entry.item.value {
        CborValue::Bool => "auto v = cbor::read_bool(c);".to_string(),
        CborValue::Text => format!("auto v = cbor::read_text(c, {});", limit(e.max_size)),
        CborValue::Bytes { exact: true } => format!(
            "auto v = cbor::read_bytes_exact(c, {});",
            e.length.expect("an exact byte string states its length")
        ),
        CborValue::Bytes { exact: false } => {
            format!("auto v = cbor::read_bytes(c, {});", limit(e.max_size))
        }
        CborValue::Enum { .. } => {
            let SceType::Enum(r) = leaf(e) else {
                unreachable!("an enum entry is of an enum type")
            };
            let carrier = crate::forge::cbor_codec::enum_carrier(imports, &r.alias)
                .expect("the import resolved the enum's carrier");
            let carrier = l.type_name(&carrier);
            let made = format!(
                "{}(static_cast<{carrier}>(*raw))",
                l.enum_from_underlying(&r.alias)
            );
            let v = if l.enum_is_open(&r.alias) {
                format!("std::optional<{}> v = {made};", entry.read)
            } else {
                format!("auto v = {made};")
            };
            format!(
                "const auto raw = cbor::read_uint_upto(c, static_cast<std::uint64_t>(std::numeric_limits<{carrier}>::max()));\n\
                 \x20               if (!raw) return std::nullopt;\n\
                 \x20               {v}"
            )
        }
        CborValue::Uint { wide: true } => "auto v = cbor::read_uint(c);".to_string(),
        CborValue::Uint { wide: false } => format!(
            "const auto raw = cbor::read_uint_upto(c, static_cast<std::uint64_t>(std::numeric_limits<{ty}>::max()));\n\
             \x20               if (!raw) return std::nullopt;\n\
             \x20               std::optional<{ty}> v = static_cast<{ty}>(*raw);",
            ty = entry.read
        ),
        CborValue::RecordList { .. } => no_record_list(),
    };
    format!(
        "                {read}\n\
         \x20               if (!v) return std::nullopt;\n\
         \x20               {name} = std::move(*v);\n",
        name = entry.name
    )
}

/// The text of the printed CBOR codec `plan` inside its namespace: the
/// readers of its entries, the trait a source meets, the encode over any
/// source, and the value, made by its constructor and read by its accessors.
pub(in super::super) fn render(
    plan: &CborPlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> String {
    let entries: Vec<Entry<'_>> = plan.entries.iter().map(|item| entry(l, item)).collect();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    let mut out = String::new();

    out.push_str(&format!(
        "/// The entries a value of `{struct_name}` presents, each read by name: the\n\
         /// member function where the type has one, otherwise a function the type's own\n\
         /// namespace declares (docs/adr/0081, step 4).\n\
         namespace source {{\n{}}}  // namespace source\n\n",
        source_readers(&names)
    ));

    let read = |e: &Entry<'_>| format!("source::{}(std::declval<const S&>())", e.name);
    let reads: Vec<String> = entries
        .iter()
        .map(|e| format!("decltype({})", read(e)))
        .collect();
    let converts: Vec<String> = entries
        .iter()
        .map(|e| {
            format!(
                "std::is_convertible<decltype({}), {}>",
                read(e),
                e.presented()
            )
        })
        .collect();
    let nothrow: Vec<String> = entries
        .iter()
        .map(|e| {
            format!(
                "noexcept(::SCE::Forge::Detail::converts_to<{}>({}))",
                e.presented(),
                read(e)
            )
        })
        .collect();
    out.push_str(&format!(
        "/// Whether `S` presents every entry of `{struct_name}`, each convertible to\n\
         /// its type (a C++17 trait, ADR 0059 §3).\n\
         template <class S, class = void>\n\
         struct is_source : std::false_type {{}};\n\
         template <class S>\n\
         struct is_source<S, std::void_t<{}>>\n\
         \x20   : std::conjunction<{}> {{}};\n\n\
         /// Whether reading every entry of a source `S` throws nothing.\n\
         template <class S, class = void>\n\
         struct is_nothrow_source : std::false_type {{}};\n\
         template <class S>\n\
         struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>\n\
         \x20   : std::bool_constant<{}> {{}};\n\n",
        reads.join(", "),
        converts.join(", "),
        nothrow.join(" && ")
    ));

    // The encode: each entry read once and taken as what it is presented as,
    // the map head counting the ones present, then each in ascending key
    // order.
    let mut encode = String::new();
    for e in &entries {
        encode.push_str(&format!(
            "        const auto& _r_{name} = source::{name}(src);\n\
             \x20       const {ty} _e_{name} = _r_{name};\n",
            name = e.name,
            ty = e.presented()
        ));
    }
    let required = plan.required();
    let optional: Vec<&Entry<'_>> = entries.iter().filter(|e| !e.required()).collect();
    if optional.is_empty() {
        encode.push_str(&format!(
            "        const std::uint64_t _count = {required};\n"
        ));
    } else {
        encode.push_str(&format!("        std::uint64_t _count = {required};\n"));
        for e in &optional {
            encode.push_str(&format!("        if (_e_{}) ++_count;\n", e.name));
        }
    }
    encode.push_str("        if (auto _e = cbor::write_map_head(w, _count); _e) return _e;\n");
    for &index in &plan.by_key {
        let e = &entries[index];
        let (open, value) = if e.required() {
            ("        {\n".to_string(), format!("_e_{}", e.name))
        } else {
            (
                format!("        if (_e_{}) {{\n", e.name),
                format!("*_e_{}", e.name),
            )
        };
        encode.push_str(&format!(
            "{open}\
             \x20           const auto& v = {value};\n\
             \x20           if (auto _e = cbor::write_uint(w, {key}); _e) return _e;\n\
             \x20           {write}\n\
             \x20       }}\n",
            key = e.item.entry.key,
            write = write_value(l, e)
        ));
    }
    out.push_str(&format!(
        "/// The encode of `{struct_name}` over any source of it: the map head counting\n\
         /// the entries present, then each in ascending key order, every head in its\n\
         /// shortest form (RFC 8949 §4.2.1). It refuses nothing; what it returns is\n\
         /// the sink's own failure, and it throws only what a read of `src` throws.\n\
         struct Wire {{\n\
         \x20   template <class S>\n\
         \x20   [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {{\n\
         \x20       static_assert(is_source<S>::value, \"S presents every entry of {struct_name}: {}\");\n\
         \x20       namespace cbor = ::SCE::Forge::Cbor;\n\
         {encode}\
         \x20       return std::nullopt;\n\
         \x20   }}\n\
         }};\n\n",
        names.join(", ")
    ));

    out.push_str(&value_class(l, imports, &entries, struct_name));
    out
}

/// The value: made by its constructor, each entry read by its accessor,
/// decoded from a map and encoded as any source is.
fn value_class(
    l: &LangCtx,
    imports: &[ImportContext],
    entries: &[Entry<'_>],
    struct_name: &str,
) -> String {
    let params: Vec<String> = entries
        .iter()
        .map(|e| format!("{} {}", e.presented(), e.name))
        .collect();
    let inits: Vec<String> = entries
        .iter()
        .map(|e| format!("{}_({})", e.name, e.name))
        .collect();
    let owns = entries.iter().any(Entry::bounded);
    let (constexpr, noexcept) = if owns {
        ("", "")
    } else {
        ("constexpr ", " noexcept")
    };
    let mut out = format!(
        "/// One CBOR map (SCE_FORGE.md §4.6.1). Decode takes the keys in any order and\n\
         /// skips one this codec does not declare; encode writes the entries present in\n\
         /// ascending key order, every head in its shortest form. A value made by\n\
         /// default holds each required entry at its type's default and each optional\n\
         /// one absent.\n\
         struct {struct_name} {{\n\
         \x20   {struct_name}() = default;\n\
         \x20   /// The value of each entry it presents, in declaration order (ADR 0059 §5).\n\
         \x20   {constexpr}{struct_name}({}){noexcept}\n\
         \x20       : {} {{}}\n",
        params.join(", "),
        inits.join(", ")
    );
    for e in entries {
        let key = e.item.entry.key;
        let body = if e.bounded() && !e.required() {
            format!(
                "if (!{name}_) {{ return std::nullopt; }} return {view}(*{name}_);",
                name = e.name,
                view = e.view
            )
        } else {
            format!("return {}_;", e.name)
        };
        let constexpr = if e.bounded() { "" } else { "constexpr " };
        let doc = if e.required() {
            format!("The entry with key {key}.")
        } else {
            format!("The entry with key {key}, where the map holds it.")
        };
        out.push_str(&format!(
            "    /// {doc}\n\
             \x20   [[nodiscard]] {constexpr}{ty} {name}() const noexcept {{ {body} }}\n",
            ty = e.presented(),
            name = e.name
        ));
    }

    // The decode: each entry read once in whatever order the map holds it,
    // a key given twice refused, an unknown key skipped; a required entry
    // missing refused; a text or a byte string then held to its bound, which
    // the read already held it within.
    let mut decode = String::new();
    for e in entries {
        decode.push_str(&format!("        std::optional<{}> {};\n", e.read, e.name));
    }
    decode.push_str(
        "        for (std::uint64_t i = 0; i < *count; ++i) {\n\
         \x20           const auto key = cbor::read_uint(c);\n\
         \x20           if (!key) return std::nullopt;\n\
         \x20           switch (*key) {\n",
    );
    let mut by_key: Vec<&Entry<'_>> = entries.iter().collect();
    by_key.sort_by_key(|e| e.item.entry.key);
    for e in &by_key {
        decode.push_str(&format!(
            "            case {key}: {{\n\
             \x20               // A key given twice is not one map entry.\n\
             \x20               if ({name}.has_value()) return std::nullopt;\n\
             {read}\
             \x20               break;\n\
             \x20           }}\n",
            key = e.item.entry.key,
            name = e.name,
            read = read_value(l, imports, e)
        ));
    }
    decode.push_str(
        "            default:\n\
         \x20               if (!cbor::skip(c)) return std::nullopt;\n\
         \x20               break;\n\
         \x20           }\n\
         \x20       }\n",
    );
    for e in entries.iter().filter(|e| e.required()) {
        decode.push_str(&format!(
            "        if (!{}.has_value()) return std::nullopt;\n",
            e.name
        ));
    }
    let mut args: Vec<String> = Vec::new();
    for e in entries {
        if !e.bounded() {
            args.push(if e.required() {
                format!("/*{name}=*/*{name}", name = e.name)
            } else {
                format!("/*{name}=*/{name}", name = e.name)
            });
            continue;
        }
        if e.required() {
            decode.push_str(&format!(
                "        const auto _b_{name} = {view}::make(*{name});\n\
                 \x20       if (!_b_{name}) return std::nullopt;\n",
                name = e.name,
                view = e.view
            ));
            args.push(format!("/*{name}=*/*_b_{name}", name = e.name));
        } else {
            decode.push_str(&format!(
                "        std::optional<{view}> _b_{name};\n\
                 \x20       if ({name}) {{\n\
                 \x20           _b_{name} = {view}::make(*{name});\n\
                 \x20           if (!_b_{name}) return std::nullopt;\n\
                 \x20       }}\n",
                name = e.name,
                view = e.view
            ));
            args.push(format!("/*{name}=*/_b_{name}", name = e.name));
        }
    }
    out.push_str(&format!(
        "\n\
         \x20   /// Decode one map from `cursor`. On success the cursor advances past it;\n\
         \x20   /// `std::nullopt` on any refusal, with the cursor left where it was.\n\
         \x20   static std::optional<{struct_name}> decode(::SCE::Forge::SceCursor& cursor) {{\n\
         \x20       ::SCE::Forge::SceCursor c = cursor;\n\
         \x20       auto value = decode_at(c);\n\
         \x20       if (value) {{\n\
         \x20           cursor = c;\n\
         \x20       }}\n\
         \x20       return value;\n\
         \x20   }}\n\n\
         \x20   /// Encode this map into `w`: the encode over any source, this value one of\n\
         \x20   /// them. It refuses nothing; what it returns is the sink's own failure.\n\
         \x20   [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {{\n\
         \x20       return Wire::encode_source(*this, w);\n\
         \x20   }}\n\n\
         \x20   /// `encode` into a new vector.\n\
         \x20   [[nodiscard]] std::vector<std::uint8_t> encode_to_vec() const {{\n\
         \x20       std::vector<std::uint8_t> out;\n\
         \x20       ::SCE::Forge::VectorSink sink(out);\n\
         \x20       (void)encode(sink);\n\
         \x20       return out;\n\
         \x20   }}\n\n\
         private:\n\
         \x20   static std::optional<{struct_name}> decode_at(::SCE::Forge::SceCursor& c) {{\n\
         \x20       namespace cbor = ::SCE::Forge::Cbor;\n\
         \x20       const auto count = cbor::read_map_len(c);\n\
         \x20       if (!count) return std::nullopt;\n\
         {decode}\
         \x20       return {struct_name}{{\n\
         {}\
         \x20       }};\n\
         \x20   }}\n\n",
        args.iter()
            .map(|a| format!("            {a},\n"))
            .collect::<String>()
    ));
    for e in entries {
        let init = if e.init.is_empty() || !e.required() {
            "{}".to_string()
        } else {
            format!("{{{}}}", e.init)
        };
        out.push_str(&format!("    {} {}_{init};\n", e.held_member(), e.name));
    }
    out.push_str("};\n");
    out
}
