// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust source interface of a codec (docs/adr/0058): `{Codec}Source`, the
//! trait a value implements to say what it presents, and the one encode that
//! reads it.
//!
//! A codec's grammar is an interface. What a value of it says — its *presented*
//! elements, in [`codec_roles`](crate::forge::codec_roles) — is a method each;
//! what the encoding determines from those (a length, a count, the flag a gated
//! field is announced by) is not a method but computed by the encode, as the
//! encoding rules of ASN.1 compute a length and never take it from the value.
//! `{Codec}::write_source` is that encode, written once against the interface;
//! the builder `{Codec}` implements it, so a value built the way it always was
//! is encoded through it, and so is a value that holds the parts in its own
//! types.
//!
//! Every statement here is the Rust statement the codec's own `encode` writes for
//! the same wire, with its reads made through the interface and its derived
//! elements computed in a prologue, before the first byte: so an encode that
//! refuses a value refuses it with the sink untouched.

use super::*;
use crate::forge::codec_roles::{
    self, BindSite, Binder, CodecRoles, Derivation, ElementId, InputSource, InputValue, Role,
};
use crate::forge::codec_totality::{self, child_alias, RefusalKind, Totality};

pub(super) mod cbor;
pub(super) mod content_line;

/// Why a codec has no source interface (yet): the shape it holds is one the
/// interface does not cover. Scaffolding for the order the shapes land in; the
/// interface covers every codec when the last of them has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Unsupported(pub(super) String);

fn no<T>(why: impl Into<String>) -> Result<T, Unsupported> {
    Err(Unsupported(why.into()))
}

/// What the interface of one codec is made from.
pub(super) struct Input<'a> {
    pub(super) m: &'a CodecModel,
    pub(super) imports: &'a [ImportContext],
    pub(super) l: &'a LangCtx,
    /// `to_pascal_case(m.name)`: the builder's name.
    pub(super) struct_name: &'a str,
    /// `<'a>` when the builder carries a lifetime, otherwise nothing.
    pub(super) struct_lifetime: &'a str,
    /// The lifetime of the codec's own view (`<'a>` or nothing), or `None` when
    /// it has none: views were not asked for, or the view is the builder.
    pub(super) view_lifetime: Option<&'a str>,
    /// Whether the codec has an owned form: it holds a borrowed field.
    pub(super) owned: bool,
    /// Whether the functions that write are always inlined into their callers
    /// (`generate --inline-encode`): the nine small functions of a frame become
    /// one, for the instructions and the call overhead they cost, at the price of
    /// text. Off, the output is as it was.
    pub(super) inline_encode: bool,
}

/// What a function of a codec that writes into the sink `sink` returns, with
/// the bound it needs (docs/adr/0065 §4): the sink's error alone for a codec
/// whose encode is total, and `CodecError`, which the sink's error joins, for
/// one that can refuse.
pub(super) fn write_result(may_fail: bool, sink: &str) -> String {
    if may_fail {
        format!("Result<(), CodecError> where CodecError: From<{sink}::Error>")
    } else {
        format!("Result<(), {sink}::Error>")
    }
}

/// `count_source`: the bytes a codec's `write_source` writes for `src`, run into
/// a sink that only counts. A codec whose encode is total counts with nothing to
/// handle; one that can refuse returns the refusal.
///
/// It is a function of its own, and the count is never run inline in a function
/// that writes into the caller's sink `K`, because such a function of a codec
/// that can refuse carries the bound `CodecError: From<K::Error>`, and Rust then
/// selects that bound for the count's `CodecError: From<Infallible>` too, which
/// it cannot satisfy. `params` and `args` are the codec's flag inputs as
/// declared and as passed on.
pub(super) fn count_function(trait_name: &str, params: &str, args: &str, may_fail: bool) -> String {
    let (ret, run, out) = if may_fail {
        (
            "Result<usize, CodecError>",
            format!("Self::write_source(src, &mut _c{args})?;"),
            "Ok(_c.len())",
        )
    } else {
        (
            "usize",
            format!("let Ok(()) = Self::write_source(src, &mut _c{args});"),
            "_c.len()",
        )
    };
    format!(
        "\x20   /// The bytes [`Self::write_source`] writes for `src`, counted with\n\
         \x20   /// nothing written.\n\
         \x20   #[doc(hidden)]\n\
         \x20   pub fn count_source<S: {trait_name} + ?Sized>(src: &S{params}) -> {ret} {{\n\
         \x20       let mut _c = ::sce_forge_runtime::codec::CountSink::new();\n\
         \x20       {run}\n\
         \x20       {out}\n\
         \x20   }}\n"
    )
}

/// The type the default arm of the codec `struct_name` holds its tag in, which
/// admits no tag the encode could not write ([`codec_totality::UnknownTag`]).
/// The codec's template defines it beside the variant, from the context
/// `render_codec` gives it under the same name.
pub(super) fn unknown_tag_type(struct_name: &str) -> String {
    format!("{struct_name}UnknownTag")
}

/// The attribute of a function that writes, for [`Input::inline_encode`].
pub(super) fn write_attribute(inline_encode: bool) -> &'static str {
    if inline_encode {
        "\x20   #[inline(always)]\n"
    } else {
        ""
    }
}

/// The source interface of a codec, as the text a codec file holds, or why the
/// codec has none yet.
pub(super) fn render(input: &Input<'_>) -> Result<String, Unsupported> {
    let plan = Plan::of(input)?;
    let text = plan.text();
    plan.totality.assert_all_written(&input.m.name);
    Ok(text)
}

/// The link from a codec's `write_source` to the counted-source witness
/// (docs/adr/0061): `{Codec}::measure`, and the impl of `SourceWrite` that lets
/// `Measured::write_into` run the codec's own skeleton into a window. It states
/// no wire rule; the one statement is `write_source`. `lt` is the codec's
/// lifetime parameter (`<'a>` or nothing). A codec that takes a flag from its
/// parent is written only through that parent and has no measure.
///
/// A codec whose encode is total states the sink's error as its own
/// (`type Error<E> = E`), so its measure, whose count cannot refuse, returns the
/// witness with no `Result` (docs/adr/0065 §4); one that can refuse states
/// `CodecError`.
pub(super) fn measure_impl(
    name: &str,
    lt: &str,
    trait_name: &str,
    inline_encode: bool,
    may_fail: bool,
) -> String {
    let generics = match lt.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
        Some(lifetime) => format!("<{lifetime}, S: {trait_name} + ?Sized>"),
        None => format!("<S: {trait_name} + ?Sized>"),
    };
    // The window is written through the sink that asks to be inlined when the
    // codec's writes are, so that the whole write can fold into the caller.
    let (inline, sink) = if inline_encode {
        (write_attribute(true), "WindowSink")
    } else {
        ("", "SliceSink")
    };
    // The trait's own `write_window` goes through `SliceSink`; a codec whose
    // writes fold into the caller says the window sink that does the same, by
    // way of the inherent function of that name (which an inherent call
    // resolves to before the trait's).
    let window_choice = if inline_encode {
        format!(
            "\n{inline}\
             \x20   fn write_window(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {{\n\
             \x20       Self::write_window(src, window)\n\
             \x20   }}\n"
        )
    } else {
        String::new()
    };
    let (error, write_error) = if may_fail {
        ("CodecError", "CodecError")
    } else {
        ("E", "K::Error")
    };
    let measure = if may_fail {
        format!(
            "\x20   /// The size of the frame [`Self::encode_source`] writes for `src`, found\n\
             \x20   /// with nothing written, or the refusal that encode would give. The result\n\
             \x20   /// writes the frame into a window of exactly that size and cannot fail.\n\
             {inline}\
             \x20   pub fn measure<S: {trait_name} + ?Sized>(src: &S) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {{\n\
             \x20       ::sce_forge_runtime::codec::measure(src)\n\
             \x20   }}\n\n"
        )
    } else {
        format!(
            "\x20   /// The size of the frame [`Self::encode_source`] writes for `src`, found\n\
             \x20   /// with nothing written: the encode refuses no value, so neither does\n\
             \x20   /// this. The result writes the frame into a window of exactly that size\n\
             \x20   /// and cannot fail.\n\
             {inline}\
             \x20   pub fn measure<S: {trait_name} + ?Sized>(src: &S) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {{\n\
             \x20       let Ok(measured) = ::sce_forge_runtime::codec::measure(src);\n\
             \x20       measured\n\
             \x20   }}\n\n"
        )
    };
    format!(
        "impl{generics} ::sce_forge_runtime::codec::SourceWrite<S> for {name}{lt} {{\n\
         \x20   type Error<E> = {error};\n\n\
         {inline}\
         \x20   fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), {write_error}> where CodecError: From<K::Error> {{\n\
         \x20       Self::write_source(src, w)\n\
         \x20   }}\n\
         {window_choice}\
         }}\n\n\
         #[allow(dead_code)]\n\
         impl{lt} {name}{lt} {{\n\
         {measure}\
         \x20   /// Write `src` into `window` in one pass and return the bytes it is, for a\n\
         \x20   /// window as large as the caller can spare. On an error the window holds a\n\
         \x20   /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].\n\
         {inline}\
         \x20   pub fn write_window<S: {trait_name} + ?Sized>(src: &S, window: &mut [u8]) -> Result<usize, CodecError> {{\n\
         \x20       let mut sink = ::sce_forge_runtime::codec::{sink}::new(window);\n\
         \x20       Self::write_source(src, &mut sink)?;\n\
         \x20       Ok(sink.position())\n\
         \x20   }}\n\
         }}\n"
    )
}

/// A representation of a codec's value: the builder a sender fills, the view of
/// a decoded frame, or the owned form that outlives it. Each says what it holds
/// through the same trait.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    Builder,
    View,
    Owned,
}

impl Form {
    /// What follows the codec's name in the name of its variant enum.
    fn variant_suffix(self) -> &'static str {
        match self {
            Form::Builder => "Variant",
            Form::View => "ViewVariant",
            Form::Owned => "OwnedVariant",
        }
    }
}

/// One arm of a codec's variant.
struct Arm {
    /// The import alias of the codec that is its body.
    alias: String,
    /// The enum variant that holds it: the body codec's name, or `Default`.
    variant_name: String,
    /// The type parameter of `{Codec}Body` that is the source of its body.
    generic: String,
    child: Child,
    /// The tag that names it; `None` for the default arm.
    value: Option<u64>,
    /// The build feature it is compiled under (docs/adr/0072); `None` for an
    /// arm every build holds, the default arm among them.
    feature: Option<String>,
}

impl Arm {
    /// The line of `#[cfg]` that compiles an item of this arm only where its
    /// feature is on, indented by `indent`; nothing for an arm every build
    /// holds.
    fn cfg_line(&self, indent: &str) -> String {
        match &self.feature {
            Some(feature) => format!("{indent}{}\n", rust_feature_cfg(feature)),
            None => String::new(),
        }
    }

    /// The same `#[cfg]` written in front of an item on its line.
    fn cfg_prefix(&self) -> String {
        match &self.feature {
            Some(feature) => format!("{} ", rust_feature_cfg(feature)),
            None => String::new(),
        }
    }
}

/// One set of the build features a variant's arms name, and the `#[cfg]`
/// predicate that holds in exactly the builds with those features on and the
/// rest off (docs/adr/0072).
struct World {
    /// `#[cfg(..)]`, or `None` when no arm names a feature and every build is
    /// the one world.
    cfg: Option<String>,
    /// The features on in this world.
    on: std::collections::BTreeSet<String>,
}

impl World {
    /// Whether `arm` is compiled in this world.
    fn holds(&self, arm: &Arm) -> bool {
        match &arm.feature {
            Some(feature) => self.on.contains(feature),
            None => true,
        }
    }
}

/// The imported codec a list or an embedded body holds.
struct Child {
    /// The codec's type, which the wire is written through (`Type::write_source`).
    type_name: String,
    /// The path of its source trait.
    trait_path: String,
    /// The path of its overrides (`{Codec}Overrides`), which a chain of the
    /// codec's entries sets the end flag through.
    overrides_path: String,
    /// The path of its borrowed source (`{Codec}Parts`) with the lifetime it
    /// takes, as a field of a parent's parts names it.
    parts_type: String,
}

/// How one field is read and written.
struct Plan<'a> {
    input: &'a Input<'a>,
    roles: CodecRoles,
    /// `{Struct}Source`.
    trait_name: String,
    /// Where the encode can refuse a value (docs/adr/0065): the one statement
    /// of it, which every refusal written here must be in.
    totality: Totality,
}

/// Whether `m` has a source interface, given whether each codec it imports has
/// one. Shared by the codec's own generation and by the enrichment of a parent
/// that imports it, so the two cannot disagree on which codecs the interface
/// covers.
pub(crate) fn supported_with(
    m: &CodecModel,
    mut import_has_source: impl FnMut(&str) -> bool,
) -> bool {
    // The answer for each codec the model holds, asked once and before the
    // check, which cannot take a closure that borrows its caller's walk.
    let known: std::collections::BTreeMap<&str, bool> = child_aliases(m)
        .into_iter()
        .map(|alias| (alias, import_has_source(alias)))
        .collect();
    check(m, &|alias| known.get(alias).copied().unwrap_or(false)).is_ok()
}

/// Whether the borrowed source of a codec (`{Codec}Parts`) carries a lifetime:
/// it holds text, bytes, a list, an embedded body or a variant, each of which it
/// borrows from the caller. A codec of scalars alone holds everything by value.
pub(crate) fn parts_has_lifetime(m: &CodecModel) -> bool {
    m.variant.is_some()
        || m.fields.iter().any(|f| {
            f.is_repeat()
                || f.is_tlv_chain()
                || f.is_embed()
                || matches!(f.sce_type, SceType::Bytes | SceType::String)
        })
}

/// The shapes the interface covers.
fn check(m: &CodecModel, import_has_source: &dyn Fn(&str) -> bool) -> Result<(), Unsupported> {
    if m.encoding != CodecEncoding::Positional {
        return no("a codec of another encoding writes its own encode");
    }
    for alias in child_aliases(m) {
        if !import_has_source(alias) {
            return no(format!(
                "the codec `{alias}` it holds has no source interface"
            ));
        }
    }
    Ok(())
}

/// Every import alias of a codec a model holds: in a list, an embedded body or
/// an arm of its variant.
fn child_aliases(m: &CodecModel) -> Vec<&str> {
    let mut aliases: Vec<&str> = m.fields.iter().filter_map(child_alias).collect();
    if let Some(variant) = &m.variant {
        aliases.extend(
            variant
                .arms
                .iter()
                .chain(variant.default_arm.iter())
                .map(|arm| arm.body_alias.as_str()),
        );
    }
    aliases
}

impl<'a> Plan<'a> {
    fn of(input: &'a Input<'a>) -> Result<Self, Unsupported> {
        let roles = codec_roles::classify(input.m, input.imports);
        let has_source = |alias: &str| {
            input
                .imports
                .iter()
                .any(|i| i.alias == alias && i.kind == "codec" && i.codec_has_source)
        };
        check(input.m, &has_source)?;
        Ok(Plan {
            input,
            roles,
            trait_name: format!("{}Source", input.struct_name),
            totality: codec_totality::analyse(
                input.m,
                &mut codec_totality::Resolved(input.imports),
            ),
        })
    }

    /// Whether the encode can refuse a value (docs/adr/0065): it then returns
    /// `CodecError`, which the sink's error joins, and otherwise the sink's
    /// error alone.
    fn may_fail(&self) -> bool {
        !self.totality.is_total()
    }

    /// The error the encode returns where it refuses at `element` with `kind`,
    /// which the totality analysis must name ([`codec_totality::Totality::require`]).
    fn refuse(&self, kind: RefusalKind, element: &str) -> String {
        self.totality.require(kind, element)
    }

    /// Whether the codec imported as `alias` can refuse a value.
    fn child_may_fail(&self, alias: &str) -> bool {
        self.input
            .imports
            .iter()
            .any(|i| i.alias == alias && i.kind == "codec" && i.codec_source_can_refuse)
    }

    /// The `?` that passes on what a codec written at `element` (the import
    /// `alias`) returns: its refusal, recorded as written, when it can refuse,
    /// and its sink's error otherwise.
    fn child_write(&self, alias: &str, element: &str) -> &'static str {
        if self.child_may_fail(alias) {
            self.refuse(RefusalKind::Child, element);
        }
        "?"
    }

    /// What a function that writes into the sink `sink` returns, with the bound
    /// it needs: the sink's error alone for a total encode, and `CodecError`,
    /// which the sink's error joins, for one that can refuse.
    fn write_result(&self, sink: &str) -> String {
        write_result(self.may_fail(), sink)
    }

    /// The imported codec a list or an embedded body holds, as the type it is
    /// written through and the trait it is read through.
    fn child(&self, field: &CodecField) -> Option<Child> {
        self.child_of(child_alias(field)?)
    }

    /// The imported codec named by `alias`.
    fn child_of(&self, alias: &str) -> Option<Child> {
        let import = self.input.imports.iter().find(|i| i.alias == alias)?;
        Some(Child {
            type_name: import.type_name.clone(),
            trait_path: format!("super::{}::{}Source", import.namespace, import.type_name),
            overrides_path: format!("super::{}::{}Overrides", import.namespace, import.type_name),
            parts_type: format!(
                "super::{}::{}Parts{}",
                import.namespace,
                import.type_name,
                if import.codec_parts_lifetime {
                    "<'a>"
                } else {
                    ""
                }
            ),
        })
    }

    /// The flags of this codec a parent may set in place of the ones a value
    /// presents: a one-bit flag of a fixed carrier that the codec presents. A
    /// chain derives the flag that says an entry is not the last, and the entry
    /// it writes cannot present that flag, as it cannot know where it stands.
    fn overridable(&self) -> Vec<(&CodecField, &FlagDef)> {
        let mut out = Vec::new();
        for carrier in &self.m().fields {
            if carrier.fixed_bits().is_none() || !self.presented(carrier) {
                continue;
            }
            for def in &carrier.flags {
                let id = ElementId::Flag {
                    field: carrier.id.clone(),
                    flag: def.name.clone(),
                };
                if def.width == 1 && matches!(self.roles.role_of(&id), Some(Role::Presented)) {
                    out.push((carrier, def));
                }
            }
        }
        out
    }

    /// The member of the entry codec's overrides that sets the flag a chain ends
    /// on, for a chain that ends on its entries' flag.
    fn chain_end_flag(&self, field: &CodecField) -> Option<String> {
        let BitSize::TlvChain {
            terminate_on: TlvTerminateStrategy::EntryFlag { flag_name },
            ..
        } = &field.bit_size
        else {
            return None;
        };
        let import = self
            .input
            .imports
            .iter()
            .find(|i| Some(i.alias.as_str()) == child_alias(field))?;
        let carrier = import
            .codec_fields
            .iter()
            .find(|f| f.flags.iter().any(|d| &d.name == flag_name))?;
        Some(self.override_member(carrier, flag_name))
    }

    /// The name of the member of `{Codec}Overrides` that sets `flag` of `carrier`.
    fn override_member(&self, carrier: &CodecField, flag: &str) -> String {
        format!(
            "{}_{}",
            self.accessor(carrier),
            filters::to_snake_case(flag.to_string())
        )
    }

    fn overrides_name(&self) -> String {
        format!("{}Overrides", self.name())
    }

    fn overrides_definition(&self) -> String {
        let overridable = self.overridable();
        if overridable.is_empty() {
            return String::new();
        }
        let members: String = overridable
            .iter()
            .map(|(carrier, def)| {
                format!(
                    "    /// The `{flag}` flag of `{carrier}`, when a parent sets it.\n    pub {member}: Option<bool>,\n",
                    flag = def.name,
                    carrier = carrier.id,
                    member = self.override_member(carrier, &def.name),
                )
            })
            .collect();
        // One constructor for each flag: the chain sets one and leaves the rest,
        // which a struct literal says only with an update that clippy refuses
        // for a struct of one member.
        let setters: String = overridable
            .iter()
            .map(|(carrier, def)| {
                let member = self.override_member(carrier, &def.name);
                format!(
                    "    /// Sets `{flag}` of `{carrier}` and nothing else.\n    #[allow(clippy::needless_update)]\n    pub fn {member}(value: bool) -> Self {{\n        Self {{ {member}: Some(value), ..Self::default() }}\n    }}\n",
                    flag = def.name,
                    carrier = carrier.id,
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "/// Flags of `{name}` a parent sets in place of the ones a value presents:\n\
             /// the end flag of a chain whose entries are `{name}`, which the chain\n\
             /// derives from where an entry stands.\n\
             #[doc(hidden)]\n\
             #[derive(Debug, Default, Clone, Copy)]\n\
             pub struct {overrides} {{\n{members}}}\n\n\
             #[allow(dead_code)]\n\
             impl {overrides} {{\n{setters}}}\n\n",
            name = self.name(),
            overrides = self.overrides_name(),
        )
    }

    /// Whether a codec the value holds (in a list, an embedded body or a
    /// variant) can refuse it: a refusal that comes from inside one has to be
    /// found before the first byte is written. A codec whose children cannot
    /// refuse has nothing to find, and skips the pass that looks. That is all
    /// the refusal analysis decides: what a caller sees is a `Result` for every
    /// codec.
    fn children_can_refuse(&self) -> bool {
        child_aliases(self.m()).into_iter().any(|alias| {
            self.input
                .imports
                .iter()
                .any(|i| i.alias == alias && i.codec_source_can_refuse)
        })
    }

    // ── flag-inputs ──────────────────────────────────────────────────

    /// The parameter a flag-input is passed as.
    fn input_name(input: &FlagInput) -> String {
        filters::to_snake_case(input.name.clone())
    }

    /// The flag-inputs the encode is handed: those the value does not say
    /// (docs/adr/0065 §6 R4a).
    fn given_inputs(&self) -> impl Iterator<Item = &FlagInput> {
        self.m()
            .flag_inputs
            .iter()
            .filter(|input| !self.roles.says_input(&input.name))
    }

    /// `, n: u8` for each flag-input the encode takes from its parent: a bit of
    /// the parent's own header that gates a field here, which the value does
    /// not say.
    fn input_params(&self) -> String {
        self.given_inputs()
            .map(|input| format!(", {}: u8", Self::input_name(input)))
            .collect()
    }

    /// `, n` for each flag-input the encode takes, passed on.
    fn input_args(&self) -> String {
        self.given_inputs()
            .map(|input| format!(", {}", Self::input_name(input)))
            .collect()
    }

    /// The truth of the flag-input `name` the value says, as an expression
    /// over `value` (a reference to a source of this codec).
    fn said_input(&self, name: &str, value: &str) -> String {
        match &self.roles.inputs[name] {
            InputSource::Gate { field, when_clear } => {
                let field = self
                    .m()
                    .fields
                    .iter()
                    .find(|f| &f.id == field)
                    .expect("an input is said by a field of the codec");
                let present = format!(
                    "{}::{}({value}).is_some()",
                    self.trait_name,
                    self.accessor(field)
                );
                if *when_clear {
                    format!("!{present}")
                } else {
                    present
                }
            }
            InputSource::Forward {
                embed,
                alias,
                input,
            } => {
                let field = self
                    .m()
                    .fields
                    .iter()
                    .find(|f| &f.id == embed)
                    .expect("an input is said by a body of the codec");
                let child = self
                    .child_of(alias)
                    .expect("`check` found the codec a body holds");
                format!(
                    "{}::input_{}({}::{}({value}))",
                    child.type_name,
                    filters::to_snake_case(input.clone()),
                    self.trait_name,
                    self.accessor(field)
                )
            }
        }
    }

    /// `input_{i}`: each flag-input the value says, read off a source
    /// (docs/adr/0065 §6 R4a). A parent derives a flag it binds to the input
    /// from it, and holds a value it hands the input from elsewhere to it.
    fn input_functions(&self) -> String {
        let mut out = String::new();
        for input in &self.m().flag_inputs {
            if !self.roles.says_input(&input.name) {
                continue;
            }
            let by = match &self.roles.inputs[&input.name] {
                InputSource::Gate {
                    field,
                    when_clear: false,
                } => format!("set exactly when `{field}` is present"),
                InputSource::Gate {
                    field,
                    when_clear: true,
                } => format!("set exactly when `{field}` is absent"),
                InputSource::Forward { embed, input, .. } => {
                    format!("the input `{input}` that the body `{embed}` says")
                }
            };
            out.push_str(&format!(
                "\x20   /// The flag-input `{name}` that `src` says: {by}.\n\
                 \x20   /// A parent hands the decode this bit and derives it from here for the\n\
                 \x20   /// encode.\n\
                 \x20   pub fn input_{snake}<S: {trait_name} + ?Sized>(src: &S) -> bool {{\n\
                 \x20       {expr}\n\
                 \x20   }}\n\n",
                name = input.name,
                snake = Self::input_name(input),
                trait_name = self.trait_name,
                expr = self.said_input(&input.name, "src"),
            ));
        }
        out
    }

    /// Whether the wire reads the flag-input `name` by name: a field gated on it
    /// that the value does not present is written where the gate holds, and a
    /// held codec handed the input is held to it.
    fn gates_read_input(&self, name: &str) -> bool {
        let gated = self.m().fields.iter().any(|f| {
            !self.presented(f)
                && f.present_if.as_ref().is_some_and(|gate| {
                    let mut clause = Some(gate);
                    while let Some(p) = clause {
                        if p.scope == PresentIfScope::Input && p.flag_name == name {
                            return true;
                        }
                        clause = p.or_with.as_deref();
                    }
                    false
                })
        });
        gated
            || self
                .roles
                .input_checks
                .iter()
                .any(|c| matches!(&c.against, InputValue::Input { name: n } if n == name))
    }

    /// Hold the input a held codec says to the value the parent hands it from
    /// elsewhere ([`codec_roles::InputCheck`]), in the prologue, so a refusal
    /// leaves the sink untouched.
    fn input_check(
        &self,
        binder: &Binder,
        against: &InputValue,
        derived: &Derived,
        lines: &mut Vec<String>,
    ) {
        let child = self
            .child_of(&binder.alias)
            .expect("`check` found the codec a bind names");
        let says = |entry: &str| {
            format!(
                "{}::input_{}({entry})",
                child.type_name,
                filters::to_snake_case(binder.input.clone())
            )
        };
        let element = binder.site.element(&binder.alias);
        let refuse = self.refuse(RefusalKind::PresentIfMismatch, element);
        let field_of = |id: &str| {
            self.m()
                .fields
                .iter()
                .find(|f| f.id == id)
                .expect("a bind site is a field of the codec")
        };
        let handed = match against {
            InputValue::Carrier { carrier, flag } => {
                let carrier = field_of(carrier);
                let def = carrier
                    .flags
                    .iter()
                    .find(|d| &d.name == flag)
                    .expect("the parser found the flag a bind names");
                format!(
                    "({} & 0x{:X}) != 0",
                    self.read(carrier, derived),
                    1u64 << def.bit
                )
            }
            InputValue::Input { name } => format!("{} != 0", filters::to_snake_case(name.clone())),
            InputValue::PreviousEntry { id } => {
                let BindSite::List { field } = &binder.site else {
                    unreachable!("a chain supplies an input from the entry before");
                };
                let chain = field_of(field);
                let accessor = self.accessor(chain);
                let walk = [
                    format!("    if {} != _prev {{", says("&_e")),
                    format!("        return Err({refuse});"),
                    "    }".to_string(),
                    format!(
                        "    _prev = {} == {id}u64;",
                        self.entry_identifier(chain, "&_e")
                    ),
                    "}".to_string(),
                ];
                lines.push("{".to_string());
                lines.push("    let mut _prev: bool = false;".to_string());
                let entries = if chain.present_if.is_some() {
                    format!("src.{accessor}().into_iter().flatten()")
                } else {
                    format!("src.{accessor}()")
                };
                lines.push(format!("    for _e in {entries} {{"));
                lines.extend(walk.iter().map(|line| format!("    {line}")));
                lines.push("}".to_string());
                return;
            }
        };
        let differs = match &binder.site {
            BindSite::List { field } => {
                let list = field_of(field);
                let accessor = self.accessor(list);
                let differs = format!("{} != _h", says("&_e"));
                if list.present_if.is_some() {
                    format!("src.{accessor}().is_some_and(|mut _l| _l.any(|_e| {differs}))")
                } else {
                    format!("src.{accessor}().any(|_e| {differs})")
                }
            }
            BindSite::Embed { field } => {
                let body = field_of(field);
                let accessor = self.accessor(body);
                if body.present_if.is_some() {
                    format!("src.{accessor}().is_some_and(|_b| {} != _h)", says("_b"))
                } else {
                    format!("{} != _h", says(&format!("src.{accessor}()")))
                }
            }
            BindSite::Arm => {
                let enum_name = self.body_enum();
                let mut arms: Vec<String> = self
                    .arms()
                    .iter()
                    .filter(|arm| arm.alias == binder.alias)
                    .map(|arm| {
                        let pattern = if arm.value.is_some() {
                            format!("{enum_name}::{}(_b)", arm.variant_name)
                        } else {
                            format!("{enum_name}::Default {{ body: _b, .. }}")
                        };
                        format!("{}{pattern} => {} != _h,", arm.cfg_prefix(), says("_b"))
                    })
                    .collect();
                arms.push("_ => false,".to_string());
                format!("match src.body() {{ {} }}", arms.join(" "))
            }
        };
        lines.push(format!(
            "{{\n    let _h: bool = {handed};\n    if {differs} {{\n        return Err({refuse});\n    }}\n}}"
        ));
    }

    /// Whether the import `alias` says its input `input`, so that it is not
    /// handed one.
    fn child_says_input(&self, alias: &str, input: &str) -> bool {
        self.input
            .imports
            .iter()
            .any(|i| i.alias == alias && i.codec_says_inputs.iter().any(|s| s == input))
    }

    /// The arguments a parent passes the codec `alias`, one for each flag-input
    /// it takes, in the order it declares them: the bits of the parent's own
    /// carrier that the import binds, or the parent's own input passed on.
    fn thread_args(&self, alias: &str, derived: &Derived) -> String {
        self.thread_args_supplying(alias, derived, &|_| None)
    }

    /// [`Self::thread_args`] for a caller that supplies some inputs itself: a
    /// chain passes the entry it writes an input of its own making, in place of
    /// the one the import binds.
    fn thread_args_supplying(
        &self,
        alias: &str,
        derived: &Derived,
        supplied: &dyn Fn(&str) -> Option<String>,
    ) -> String {
        let Some(import) = self.input.imports.iter().find(|i| i.alias == alias) else {
            return String::new();
        };
        let mut out = String::new();
        for input in &import.codec_flag_inputs {
            // An input the codec says is not handed to it.
            if self.child_says_input(alias, &input.name) {
                continue;
            }
            if let Some(arg) = supplied(&input.name) {
                out.push_str(&format!(", {arg}"));
                continue;
            }
            let Some(bind) = import.flag_binds.iter().find(|b| b.input == input.name) else {
                continue;
            };
            match &bind.source {
                FlagBindSource::Carrier { carrier, flag } => {
                    let Some(field) = self
                        .m()
                        .fields
                        .iter()
                        .find(|f| f.is_flags_carrier() && &f.id == carrier)
                    else {
                        continue;
                    };
                    let Some(def) = field.flags.iter().find(|d| &d.name == flag) else {
                        continue;
                    };
                    let mask = (1u64 << def.width.max(1)) - 1;
                    let read = self.read(field, derived);
                    let bits = format!("{} & 0x{mask:X}", shr(&read, def.bit));
                    if field.fixed_bits().is_some_and(|b| b <= 8) {
                        out.push_str(&format!(", {bits}"));
                    } else {
                        out.push_str(&format!(", ({bits}) as u8"));
                    }
                }
                FlagBindSource::Input { name } => {
                    out.push_str(&format!(", {}", filters::to_snake_case(name.clone())));
                }
            }
        }
        out
    }

    // ── gates that are checked ───────────────────────────────────────

    /// The test a gate states, over what the source presents and what the
    /// codec is passed: each clause reads a bit, and the clauses are joined
    /// as the author joined them.
    fn gate_test(&self, gate: &PresentIfPredicate, derived: &Derived) -> String {
        let mut clauses: Vec<String> = Vec::new();
        let mut clause = Some(gate);
        while let Some(p) = clause {
            if p.scope == PresentIfScope::Chain {
                // The chain holds an entry of the identifier, as the entries
                // the source lists say.
                let has = self.chain_has(p);
                clauses.push(if p.negate { format!("!{has}") } else { has });
                clause = p.or_with.as_deref();
                continue;
            }
            let (read, mask) = match p.scope {
                PresentIfScope::Input => (filters::to_snake_case(p.flag_name.clone()), 1u64),
                _ => {
                    let carrier = self
                        .m()
                        .fields
                        .iter()
                        .find(|f| f.id == p.field_id)
                        .expect("the parser found the carrier a gate reads");
                    let def = carrier
                        .flags
                        .iter()
                        .find(|d| d.name == p.flag_name)
                        .expect("the parser found the flag a gate reads");
                    (self.read(carrier, derived), 1u64 << def.bit)
                }
            };
            let op = if p.negate { "==" } else { "!=" };
            clauses.push(format!("({read} & 0x{mask:X}) {op} 0"));
            clause = p.or_with.as_deref();
        }
        clauses.join(" || ")
    }

    /// The identifier of the entry `entry` (an expression that is a reference
    /// to the entry's source) of the chain `chain`, as a `u64`: the flag or
    /// field the chain's `entry-id` names, read through the entry codec's trait,
    /// less the flags it leaves out.
    fn entry_identifier(&self, chain: &CodecField, entry: &str) -> String {
        let BitSize::TlvChain {
            entry_id: Some(identifier),
            ..
        } = &chain.bit_size
        else {
            panic!("a chain-membership clause names a chain that declares entry-id");
        };
        let import = self
            .input
            .imports
            .iter()
            .find(|i| Some(i.alias.as_str()) == child_alias(chain))
            .expect("a chain imports its entry codec");
        let entry_trait = format!("super::{}::{}Source", import.namespace, import.type_name);
        match &identifier.carrier {
            Some(carrier_id) => {
                let carrier = import
                    .codec_fields
                    .iter()
                    .find(|f| &f.id == carrier_id)
                    .expect("the parser found the carrier the identifier names");
                let def = carrier
                    .flags
                    .iter()
                    .find(|d| d.name == identifier.name)
                    .expect("the parser found the flag the identifier names");
                let mask = (1u64 << def.width) - 1;
                let whole = format!(
                    "u64::from({entry_trait}::{}({entry}))",
                    self.l().codec_field_id(&carrier.id)
                );
                format!("({} & 0x{mask:X})", shr(&whole, def.bit))
            }
            None => {
                let whole = format!(
                    "u64::from({entry_trait}::{}({entry}))",
                    self.l().codec_field_id(&identifier.name)
                );
                match chain_identifier_masks(self.m(), self.input.imports).get(&chain.id) {
                    Some(mask) => format!("({whole} & 0x{mask:X})"),
                    None => whole,
                }
            }
        }
    }

    /// "The chain holds an entry whose identifier is the clause's".
    fn chain_has(&self, clause: &PresentIfPredicate) -> String {
        let chain = self
            .m()
            .fields
            .iter()
            .find(|f| f.id == clause.field_id)
            .expect("the parser found the chain a gate reads");
        let value = clause.entry_id.expect("a chain clause names an identifier");
        let found = format!("{} == {value}u64", self.entry_identifier(chain, "&_e"));
        let accessor = self.accessor(chain);
        // A chain that is gated off holds no entry.
        if chain.present_if.is_some() {
            format!("src.{accessor}().is_some_and(|mut _l| _l.any(|_e| {found}))")
        } else {
            format!("src.{accessor}().any(|_e| {found})")
        }
    }

    /// Each gate the codec cannot derive the presence from, held to the source:
    /// the field is present exactly when the gate says, or the value is not one
    /// frame can carry. Made in the prologue, so nothing is written first.
    fn gate_checks(&self, derived: &Derived, lines: &mut Vec<String>) {
        for check in &self.roles.input_checks {
            self.input_check(&check.binder, &check.against, derived, lines);
        }
        for check in self.roles.source_checks() {
            let Some(field) = self.m().fields.iter().find(|f| f.id == check.field) else {
                continue;
            };
            let Some(gate) = &field.present_if else {
                continue;
            };
            lines.push(format!(
                "if ({}) != src.{}().is_some() {{\n    return Err({});\n}}",
                self.gate_test(gate, derived),
                self.accessor(field),
                self.refuse(RefusalKind::PresentIfMismatch, &field.id),
            ));
        }
    }

    fn m(&self) -> &CodecModel {
        self.input.m
    }

    fn l(&self) -> &LangCtx {
        self.input.l
    }

    fn name(&self) -> &str {
        self.input.struct_name
    }

    fn lt(&self) -> &str {
        self.input.struct_lifetime
    }

    /// The accessor a field is read through.
    fn accessor(&self, field: &CodecField) -> String {
        self.l().codec_field_id(&field.id)
    }

    /// The local that holds the value of a derived element, named after it.
    fn local(&self, prefix: &str, id: &ElementId) -> String {
        let name = match id {
            ElementId::Field(field) => self.l().codec_field_id(field),
            ElementId::Flag { field, flag } => format!(
                "{}_{}",
                self.l().codec_field_id(field),
                filters::to_snake_case(flag.clone())
            ),
            ElementId::Variant => "variant".to_string(),
        };
        format!("{prefix}{name}")
    }

    /// The local that holds a carrier with its derived flags folded in.
    fn carrier_local(&self, field: &CodecField) -> String {
        format!("_c_{}", self.l().codec_field_id(&field.id))
    }

    /// Whether a source presents `field`: it is not derived whole
    /// ([`codec_totality::presented`]).
    fn presented(&self, field: &CodecField) -> bool {
        codec_totality::presented(&self.roles, field)
    }

    /// The fields whose size the element `id` is derived from, when it is a
    /// length or a count ([`codec_totality::measured`]).
    fn measured(&self, id: &ElementId) -> Vec<&CodecField> {
        codec_totality::measured(self.m(), &self.roles, id)
    }

    /// Whether a source states `field` when its content is absent: derived only
    /// while the fields it measures are there ([`codec_totality::stated`]).
    fn stated(&self, field: &CodecField) -> bool {
        codec_totality::stated(self.m(), &self.roles, field)
    }

    /// The condition, in a form that holds its fields as members, under which all
    /// the content `field` measures is absent.
    fn absent_content(&self, field: &CodecField) -> String {
        self.measured(&ElementId::Field(field.id.clone()))
            .into_iter()
            .map(|f| format!("self.{}.is_none()", self.accessor(f)))
            .collect::<Vec<_>>()
            .join(" && ")
    }

    /// The type an accessor returns for the value of `field` when it is
    /// present: a scalar by value, bytes and text by reference.
    fn value_type(&self, field: &CodecField) -> String {
        if let Some(child) = self.child(field) {
            // An embedded body is read by reference, a list as the entries it
            // holds in order, with their count, no more than its bound
            // (docs/adr/0065 §6 R2): any iterator over values that say an entry.
            return if field.is_embed() {
                format!("&impl {}", child.trait_path)
            } else {
                format!(
                    "{RUNTIME_CODEC}::BoundedIter<impl Iterator<Item = impl {}> + Clone, {}>",
                    child.trait_path,
                    codec_totality::list_bound(field)
                )
            };
        }
        match &field.sce_type {
            SceType::Bytes => "&[u8]".to_string(),
            SceType::String => "&str".to_string(),
            other => self.l().type_name(other).to_string(),
        }
    }

    fn accessor_type(&self, field: &CodecField) -> String {
        let value = self.value_type(field);
        // A gated field may be absent; an element stated only while its content
        // is absent is absent the rest of the time.
        if field.present_if.is_some() || self.stated(field) {
            format!("Option<{value}>")
        } else {
            value
        }
    }

    // ── the interface ────────────────────────────────────────────────

    // ── the variant ──────────────────────────────────────────────────

    /// `{Codec}VariantSource`: the arm a value holds, as the interface says it.
    fn body_enum(&self) -> String {
        format!("{}VariantSource", self.name())
    }

    /// The arms of the variant: those it enumerates, then the default.
    fn arms(&self) -> Vec<Arm> {
        let Some(variant) = &self.m().variant else {
            return Vec::new();
        };
        let mut arms: Vec<Arm> = variant
            .arms
            .iter()
            .enumerate()
            .filter_map(|(i, arm)| {
                Some(Arm {
                    alias: arm.body_alias.clone(),
                    variant_name: filters::to_pascal_case(arm.body_alias.clone()),
                    generic: format!("A{i}"),
                    child: self.child_of(&arm.body_alias)?,
                    value: Some(arm.value),
                    feature: arm.feature.clone(),
                })
            })
            .collect();
        if let Some(default) = &variant.default_arm {
            if let Some(child) = self.child_of(&default.body_alias) {
                arms.push(Arm {
                    alias: default.body_alias.clone(),
                    variant_name: "Default".to_string(),
                    generic: "D".to_string(),
                    child,
                    value: None,
                    feature: None,
                });
            }
        }
        arms
    }

    /// The builds the variant's arms tell apart (docs/adr/0072): one for each
    /// set of the features its arms name, or the single build when they name
    /// none. An item whose text names the arms a build holds — the type of the
    /// accessor, whose type parameters are one per arm — is written once per
    /// world, under that world's `#[cfg]`.
    fn worlds(&self) -> Vec<World> {
        let features: Vec<String> = self
            .arms()
            .into_iter()
            .filter_map(|arm| arm.feature)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        if features.is_empty() {
            return vec![World {
                cfg: None,
                on: std::collections::BTreeSet::new(),
            }];
        }
        (0..1usize << features.len())
            .map(|mask| {
                let on: std::collections::BTreeSet<String> = features
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| (mask >> i) & 1 == 1)
                    .map(|(_, f)| f.clone())
                    .collect();
                let terms: Vec<String> = features
                    .iter()
                    .map(|f| {
                        if on.contains(f) {
                            format!("feature = \"{f}\"")
                        } else {
                            format!("not(feature = \"{f}\")")
                        }
                    })
                    .collect();
                let predicate = match terms.as_slice() {
                    [one] => one.clone(),
                    _ => format!("all({})", terms.join(", ")),
                };
                World {
                    cfg: Some(format!("#[cfg({predicate})]")),
                    on,
                }
            })
            .collect()
    }

    /// `render(ty)` for the accessor type of the variant in each world, each
    /// under its `#[cfg]` line indented by `indent`.
    fn per_world(&self, indent: &str, render: impl Fn(&str) -> String) -> String {
        let mut out = String::new();
        for world in self.worlds() {
            if let Some(cfg) = &world.cfg {
                out.push_str(&format!("{indent}{cfg}\n"));
            }
            out.push_str(&render(&self.variant_type(&world)));
        }
        out
    }

    /// The type a tag takes: its carrier's, or the narrowest unsigned type that
    /// holds the bit-range it is read from.
    fn tag_type(&self) -> SceType {
        codec_totality::tag_type(self.m())
    }

    fn unknown_tag_type(&self) -> String {
        unknown_tag_type(self.name())
    }

    /// What the accessor of the variant returns in `world`: the enum, with the
    /// source of the body of each arm the world holds as a type of its own.
    fn variant_type(&self, world: &World) -> String {
        let args: Vec<String> = self
            .arms()
            .iter()
            .filter(|arm| world.holds(arm))
            .map(|arm| format!("impl {}", arm.child.trait_path))
            .collect();
        format!("{}<'_, {}>", self.body_enum(), args.join(", "))
    }

    fn body_enum_definition(&self) -> String {
        let arms = self.arms();
        if arms.is_empty() {
            return String::new();
        }
        // A type parameter of an arm under a build feature is declared under
        // it too, as an unused one is an error (E0392).
        let generics: Vec<String> = arms
            .iter()
            .map(|arm| format!("{}{}: ?Sized", arm.cfg_prefix(), arm.generic))
            .collect();
        let tag = self.unknown_tag_type();
        let mut variants = String::new();
        for arm in &arms {
            variants.push_str(&arm.cfg_line("    "));
            if arm.value.is_some() {
                variants.push_str(&format!("    {}(&'s {}),\n", arm.variant_name, arm.generic));
            } else {
                variants.push_str(&format!(
                    "    Default {{ tag: {tag}, body: &'s {} }},\n",
                    arm.generic
                ));
            }
        }
        format!(
            "/// The arm of `{name}`'s variant a value holds, each arm's body as the\n\
             /// source it says. The tag is not among them: `{name}` derives it from\n\
             /// the arm, as it derives a length, and a default arm presents the tag\n\
             /// it could not name.\n\
             // A default arm's tag is read by the codec that embeds this one and\n\
             // derives its own carrier bits from the arm, which not every build uses.\n\
             #[allow(dead_code)]\n\
             pub enum {enum_name}<'s, {generics}> {{\n{variants}}}\n\n",
            name = self.name(),
            enum_name = self.body_enum(),
            generics = generics.join(", "),
        )
    }

    // ── the borrowed source ──────────────────────────────────────────

    fn parts_name(&self) -> String {
        format!("{}Parts", self.name())
    }

    fn parts_body_name(&self) -> String {
        format!("{}PartsBody", self.name())
    }

    /// `<'a>` when the parts borrow from the caller, otherwise nothing.
    fn parts_lt(&self) -> &'static str {
        if parts_has_lifetime(self.m()) {
            "<'a>"
        } else {
            ""
        }
    }

    /// The type a field of the parts holds: what the caller already has in hand
    /// for the value the accessor says, borrowed for the life of the parts.
    fn parts_value_type(&self, field: &CodecField) -> String {
        if let Some(child) = self.child(field) {
            // A list is a slice no longer than its bound (docs/adr/0065 §6 R2):
            // `Bounded::new` checks a slice once, and an array within the bound
            // converts without a check.
            return if field.is_embed() {
                format!("&'a {}", child.parts_type)
            } else {
                format!(
                    "{RUNTIME_CODEC}::Bounded<&'a [{}], {}>",
                    child.parts_type,
                    codec_totality::list_bound(field)
                )
            };
        }
        match &field.sce_type {
            SceType::Bytes => "&'a [u8]".to_string(),
            SceType::String => "&'a str".to_string(),
            other => self.l().type_name(other).to_string(),
        }
    }

    fn parts_field_type(&self, field: &CodecField) -> String {
        let value = self.parts_value_type(field);
        if field.present_if.is_some() || self.stated(field) {
            format!("Option<{value}>")
        } else {
            value
        }
    }

    /// `{Codec}Parts`: a source that holds what a value says in the types a
    /// caller already has, with its lists as slices of the entries' parts and its
    /// bodies as references. Nothing is copied into it and it reserves no room
    /// for a list's capacity, so the stack an encode needs is the entries the
    /// caller holds (ADR 0058 §4).
    fn parts_definition(&self) -> String {
        let mut members = String::new();
        for field in &self.m().fields {
            if !(self.presented(field) || self.stated(field)) {
                continue;
            }
            members.push_str(&format!(
                "    /// The `{id}` field.\n    pub {accessor}: {ty},\n",
                id = field.id,
                accessor = self.accessor(field),
                ty = self.parts_field_type(field),
            ));
        }
        let mut out = String::new();
        if self.m().variant.is_some() {
            members.push_str(&format!(
                "    /// The arm of the variant this value holds.\n    pub body: {}{},\n",
                self.parts_body_name(),
                self.parts_lt()
            ));
            let mut arms = String::new();
            let tag = self.unknown_tag_type();
            for arm in self.arms() {
                arms.push_str(&arm.cfg_line("    "));
                if arm.value.is_some() {
                    arms.push_str(&format!(
                        "    {}(&'a {}),\n",
                        arm.variant_name, arm.child.parts_type
                    ));
                } else {
                    arms.push_str(&format!(
                        "    /// The tag the grammar could not name, and the body it holds.\n    Default {{ tag: {tag}, body: &'a {} }},\n",
                        arm.child.parts_type
                    ));
                }
            }
            out.push_str(&format!(
                "/// The arm of `{name}`'s variant that {parts} holds.\n\
                 #[allow(dead_code)]\n\
                 #[derive(Debug, Clone, Copy)]\n\
                 pub enum {body}<'a> {{\n{arms}}}\n\n",
                name = self.name(),
                parts = self.parts_name(),
                body = self.parts_body_name(),
            ));
        }
        out.push_str(&format!(
            "/// A value of `{name}` held in the types a caller already has: a source\n\
             /// ([`{trait_name}`]) whose lists are slices within their bounds and whose\n\
             /// bodies are references.\n\
             #[allow(dead_code)]\n\
             #[derive(Debug, Clone, Copy)]\n\
             pub struct {parts}{lt} {{\n{members}}}\n\n",
            name = self.name(),
            trait_name = self.trait_name,
            parts = self.parts_name(),
            lt = self.parts_lt(),
        ));
        out
    }

    fn parts_impl(&self) -> String {
        let mut items = String::new();
        for field in &self.m().fields {
            if !(self.presented(field) || self.stated(field)) {
                continue;
            }
            let accessor = self.accessor(field);
            let read = if self.child(field).is_some() && !field.is_embed() {
                if field.present_if.is_some() {
                    format!("self.{accessor}.as_ref().map(|_l| _l.iter())")
                } else {
                    format!("self.{accessor}.iter()")
                }
            } else {
                format!("self.{accessor}")
            };
            items.push_str(&format!(
                "    fn {accessor}(&self) -> {ty} {{\n        {read}\n    }}\n",
                ty = self.accessor_type(field),
            ));
        }
        if self.m().variant.is_some() {
            let enum_name = self.body_enum();
            let parts_body = self.parts_body_name();
            let mut arms = String::new();
            for arm in self.arms() {
                arms.push_str(&arm.cfg_line("            "));
                if arm.value.is_some() {
                    arms.push_str(&format!(
                        "            {parts_body}::{name}(_b) => {enum_name}::{name}(*_b),\n",
                        name = arm.variant_name,
                    ));
                } else {
                    arms.push_str(&format!(
                        "            {parts_body}::Default {{ tag, body }} => {enum_name}::Default {{ tag: *tag, body: *body }},\n",
                    ));
                }
            }
            items.push_str(&self.per_world("    ", |ty| {
                format!(
                    "    fn body(&self) -> {ty} {{\n        match &self.body {{\n{arms}        }}\n    }}\n"
                )
            }));
        }
        let lt = self.parts_lt();
        format!(
            "impl{lt} {trait_name} for {parts}{lt} {{\n{items}}}\n\n",
            trait_name = self.trait_name,
            parts = self.parts_name(),
        )
    }

    fn text(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.body_enum_definition());
        out.push_str(&self.overrides_definition());
        out.push_str(&self.trait_definition());
        out.push_str(&self.reference_impl());
        out.push_str(&self.parts_definition());
        out.push_str(&self.parts_impl());
        out.push_str(&self.impls());
        out.push_str(&self.encode_impl());
        out
    }

    fn trait_definition(&self) -> String {
        let mut items = String::new();
        for field in &self.m().fields {
            if !(self.presented(field) || self.stated(field)) {
                continue;
            }
            items.push_str(&format!(
                "    /// The `{id}` field.\n    fn {accessor}(&self) -> {ty};\n",
                id = field.id,
                accessor = self.accessor(field),
                ty = self.accessor_type(field),
            ));
        }
        if self.m().variant.is_some() {
            items.push_str(&self.per_world("    ", |ty| {
                format!("    /// The arm of the variant this value holds.\n    fn body(&self) -> {ty};\n")
            }));
        }
        format!(
            "// ── Source (ADR 0058) ────────────────────────────────────────────────\n\
             /// What a value of `{name}` says: its presented fields. A length, a count\n\
             /// or the flag a gated field is announced by is not among them; `{name}`\n\
             /// computes those from what is ([`{name}::encode_source`]).\n\
             pub trait {trait_name} {{\n{items}}}\n\n",
            name = self.name(),
            trait_name = self.trait_name,
        )
    }

    /// `impl<T: {Trait} + ?Sized> {Trait} for &T`, which lets a list of
    /// references, or a reference to a value, stand for the value.
    fn reference_impl(&self) -> String {
        let mut items = String::new();
        for field in &self.m().fields {
            if !(self.presented(field) || self.stated(field)) {
                continue;
            }
            let accessor = self.accessor(field);
            items.push_str(&format!(
                "    fn {accessor}(&self) -> {ty} {{\n        (**self).{accessor}()\n    }}\n",
                ty = self.accessor_type(field),
            ));
        }
        if self.m().variant.is_some() {
            items.push_str(&self.per_world("    ", |ty| {
                format!("    fn body(&self) -> {ty} {{\n        (**self).body()\n    }}\n")
            }));
        }
        format!(
            "impl<T: {trait_name} + ?Sized> {trait_name} for &T {{\n{items}}}\n\n",
            trait_name = self.trait_name,
        )
    }

    /// Every form of the codec that says what it holds: the builder, its view,
    /// and its owned form. They are three representations of one value, and the
    /// one `encode_source` writes them all.
    fn impls(&self) -> String {
        let mut out = self.impl_for(Form::Builder);
        if self.input.view_lifetime.is_some() {
            out.push_str(&self.impl_for(Form::View));
        }
        if self.input.owned {
            out.push_str(&self.impl_for(Form::Owned));
        }
        out
    }

    /// The `impl` of the source trait for one form of the codec.
    fn impl_for(&self, form: Form) -> String {
        let storage = "::sce_forge_runtime::codec";
        let mut items = String::new();
        for field in &self.m().fields {
            if !(self.presented(field) || self.stated(field)) {
                continue;
            }
            let accessor = self.accessor(field);
            let gated = field.present_if.is_some();
            // A list or an embedded body is lent, a scalar copied or reborrowed.
            // An owned form holds its text, bytes and lists in the storage
            // profile's containers, which it reads through their traits.
            let read = if self.stated(field) {
                // The number the wire holds beside content that is not there.
                format!(
                    "if {} {{ Some(self.{accessor}) }} else {{ None }}",
                    self.absent_content(field)
                )
            } else if field.is_embed() {
                if gated {
                    format!("self.{accessor}.as_ref()")
                } else {
                    format!("&self.{accessor}")
                }
            } else if self.child(field).is_some() {
                // A list's entries with their count (docs/adr/0065 §6 R2). The
                // builder's `heapless::Vec` is sized to the bound, which its type
                // proves; the view and the owned form hold a `Bounded`, the owned
                // one around its profile's list, read as a slice.
                match (form, gated) {
                    (Form::Builder, true) => {
                        format!("self.{accessor}.as_ref().map({storage}::BoundedIter::from)")
                    }
                    (Form::Builder, false) => {
                        format!("{storage}::BoundedIter::from(&self.{accessor})")
                    }
                    (Form::Owned, true) => {
                        format!("self.{accessor}.as_ref().map(|_l| _l.list_iter())")
                    }
                    (Form::Owned, false) => format!("self.{accessor}.list_iter()"),
                    (_, true) => format!("self.{accessor}.as_ref().map(|_l| _l.iter())"),
                    (_, false) => format!("self.{accessor}.iter()"),
                }
            } else if form == Form::Owned && matches!(field.sce_type, SceType::Bytes) {
                if gated {
                    format!("self.{accessor}.as_ref().map({storage}::SceByteBuf::as_slice)")
                } else {
                    format!("{storage}::SceByteBuf::as_slice(&self.{accessor})")
                }
            } else if form == Form::Owned && matches!(field.sce_type, SceType::String) {
                if gated {
                    format!("self.{accessor}.as_ref().map({storage}::SceStr::as_str)")
                } else {
                    format!("{storage}::SceStr::as_str(&self.{accessor})")
                }
            } else {
                format!("self.{accessor}")
            };
            items.push_str(&format!(
                "    fn {accessor}(&self) -> {ty} {{\n        {read}\n    }}\n",
                ty = self.accessor_type(field),
            ));
        }
        if self.m().variant.is_some() {
            // The arm the value holds, each body lent.
            let enum_name = self.body_enum();
            let variant = format!("{}{}", self.name(), form.variant_suffix());
            let mut arms = String::new();
            for arm in self.arms() {
                arms.push_str(&arm.cfg_line("            "));
                if arm.value.is_some() {
                    arms.push_str(&format!(
                        "            {variant}::{name}(_b) => {enum_name}::{name}(_b),\n",
                        name = arm.variant_name,
                    ));
                } else {
                    arms.push_str(&format!(
                        "            {variant}::Default {{ tag, body }} => {enum_name}::Default {{ tag: *tag, body }},\n",
                    ));
                }
            }
            items.push_str(&self.per_world("    ", |ty| {
                format!(
                    "    fn body(&self) -> {ty} {{\n        match &self.body {{\n{arms}        }}\n    }}\n"
                )
            }));
        }
        let head = match form {
            Form::Builder => format!(
                "impl{lt} {} for {}{lt}",
                self.trait_name,
                self.name(),
                lt = self.lt()
            ),
            Form::View => {
                let lt = self.input.view_lifetime.unwrap_or_default();
                format!("impl{lt} {} for {}View{lt}", self.trait_name, self.name())
            }
            Form::Owned => format!(
                "impl<S: {storage}::CodecStorage> {} for {}Owned<S>",
                self.trait_name,
                self.name()
            ),
        };
        format!("{head} {{\n{items}}}\n\n")
    }

    // ── the encode ───────────────────────────────────────────────────

    fn encode_impl(&self) -> String {
        let write = self.write_functions(&self.write_body());
        // A list or an embedded body is written by the codec it holds, which
        // can refuse a value after this codec has written bytes. The same write
        // into a sink that only counts finds that refusal first, so the
        // caller's sink is untouched when the encode refuses.
        let params = self.input_params();
        let args = self.input_args();
        let guard = if self.children_can_refuse() {
            format!("\x20       Self::count_source(src{args})?;\n")
        } else {
            String::new()
        };
        let count = count_function(&self.trait_name, &params, &args, self.may_fail());
        let measure = if self.given_inputs().next().is_none() {
            format!(
                "\n{}",
                measure_impl(
                    self.name(),
                    self.lt(),
                    &self.trait_name,
                    self.input.inline_encode,
                    self.may_fail(),
                )
            )
        } else {
            String::new()
        };
        let refused = if self.may_fail() {
            "\x20   /// `src` presents, and a value the encoding cannot carry is refused\n\
             \x20   /// before the first byte is written.\n"
        } else {
            "\x20   /// `src` presents. Every value `src` can present has a frame, so the\n\
             \x20   /// only error is the sink's (docs/adr/0065).\n"
        };
        format!(
            "#[allow(dead_code)]\n\
             impl{lt} {name}{lt} {{\n\
             \x20   /// Encode what `src` says into the caller-owned sink: the bytes\n\
             \x20   /// [`Self::encode`] writes for the value it holds, read through\n\
             \x20   /// [`{trait_name}`]. A length, a count or a gate flag is computed from what\n\
             {refused}\
             \x20   pub fn encode_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K{params}) -> {result} {{\n\
             {guard}\
             \x20       Self::write_source(src, w{args})\n\
             \x20   }}\n\n\
             {inputs}\
             {write}\n\
             {count}\
             }}\n\
             {measure}",
            lt = self.lt(),
            name = self.name(),
            trait_name = self.trait_name,
            result = self.write_result("K"),
            inputs = self.input_functions(),
        )
    }

    /// `write_source`: the wire without the guard a parent has already made. A
    /// codec with flags a parent may set also has `write_source_in`, which
    /// takes them, and `write_source` is that with none set.
    fn write_functions(&self, body: &str) -> String {
        let trait_name = &self.trait_name;
        let params = self.input_params();
        let args = self.input_args();
        let inline = write_attribute(self.input.inline_encode);
        let result = self.write_result("K");
        if self.overridable().is_empty() {
            return format!(
                "\x20   /// [`Self::encode_source`] without the guard a parent has already made.\n\
                 \x20   #[doc(hidden)]\n\
                 {inline}\
                 \x20   pub fn write_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K{params}) -> {result} {{\n\
                 {body}\
                 \x20   }}\n"
            );
        }
        let overrides = self.overrides_name();
        format!(
            "\x20   /// [`Self::encode_source`] without the guard a parent has already made.\n\
             \x20   #[doc(hidden)]\n\
             {inline}\
             \x20   pub fn write_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K{params}) -> {result} {{\n\
             \x20       Self::write_source_in(src, w{args}, &{overrides}::default())\n\
             \x20   }}\n\n\
             \x20   /// [`Self::write_source`] with the flags a parent sets in place of the\n\
             \x20   /// ones `src` presents.\n\
             \x20   #[doc(hidden)]\n\
             {inline}\
             \x20   pub fn write_source_in<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K{params}, over: &{overrides}) -> {result} {{\n\
             {body}\
             \x20   }}\n"
        )
    }

    /// The statements of `write_source`: the derived values, then the wire.
    fn write_body(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        // An input the value says is read off it, under the name a gate reads
        // it by; a codec that does not test it has nothing to read
        // (docs/adr/0065 §6 R4a).
        for input in &self.m().flag_inputs {
            if self.roles.says_input(&input.name) && self.gates_read_input(&input.name) {
                lines.push(format!(
                    "let {}: u8 = u8::from({});",
                    Self::input_name(input),
                    self.said_input(&input.name, "src")
                ));
            }
        }
        let derived = self.derivations(&mut lines);
        self.gate_checks(&derived, &mut lines);
        for input in self.given_inputs() {
            // A codec that does not test its input still takes it.
            lines.push(format!("let _ = {};", Self::input_name(input)));
        }
        if self.m().fields.is_empty() && self.m().variant.is_none() {
            // An empty body: the surrounding wire-protocol header alone marks the
            // codec, and nothing is read or written.
            lines.push("let _ = (src, w);".to_string());
            lines.push("Ok(())".to_string());
            return indent(&lines, 8);
        }
        if self.m().positional_layout_is_valid() {
            self.positional(&derived, &mut lines);
        } else {
            for field in &self.m().fields {
                self.streamed(field, &derived, &mut lines);
            }
        }
        self.write_variant(&derived, &mut lines);
        lines.push("Ok(())".to_string());
        indent(&lines, 8)
    }

    /// The suffix: the body of the arm the value holds.
    fn write_variant(&self, derived: &Derived, lines: &mut Vec<String>) {
        if self.m().variant.is_none() {
            return;
        }
        let enum_name = self.body_enum();
        lines.push("match src.body() {".to_string());
        for arm in self.arms() {
            let pattern = if arm.value.is_some() {
                format!("{enum_name}::{}(_b)", arm.variant_name)
            } else {
                format!("{enum_name}::Default {{ body: _b, .. }}")
            };
            // An arm under a build feature is written in every rendering, its
            // refusal recorded as any arm's (docs/adr/0065), and compiled where
            // the feature is on (docs/adr/0072).
            if let Some(feature) = &arm.feature {
                lines.push(format!("    {}", rust_feature_cfg(feature)));
            }
            lines.push(format!(
                "    {pattern} => {}::write_source(_b, w{}){},",
                arm.child.type_name,
                self.thread_args(&arm.alias, derived),
                self.child_write(&arm.alias, &arm.alias),
            ));
        }
        lines.push("}".to_string());
    }

    /// The bits of a carrier that an embedded variant's active arm fills: the
    /// arm's value, read from the embedded body's source. A gated body that is
    /// absent fills none.
    fn derive_parent_tag(&self, id: &ElementId, lines: &mut Vec<String>, derived: &mut Derived) {
        let ElementId::Flag { field, flag } = id else {
            unreachable!("a parent tag fills a flag of a carrier");
        };
        let Some(carrier) = self.m().fields.iter().find(|f| &f.id == field) else {
            return;
        };
        let ty = self.l().type_name(&carrier.sce_type).to_string();
        let local = self.local("_d_", id);
        let mut terms: Vec<String> = Vec::new();
        for d in collect_parent_tag_derivations(self.m(), self.input.imports) {
            if &d.carrier_field_id != field || &d.carrier_flag != flag {
                continue;
            }
            let Some(import) = self.input.imports.iter().find(|i| i.alias == d.embed_alias) else {
                continue;
            };
            let Some(embed) = self.m().fields.iter().find(|f| f.id == d.embed_field_id) else {
                continue;
            };
            let enum_path = format!(
                "super::{}::{}VariantSource",
                import.namespace, import.type_name
            );
            let mask: u64 = (1u64 << d.width) - 1;
            // An arm the embedded codec compiles under a build feature has no
            // variant in a build without it, so its arm here is compiled under
            // the same feature (docs/adr/0072).
            let mut arms: Vec<String> = d
                .arms
                .iter()
                .map(|arm| {
                    format!(
                        "{}{enum_path}::{}(_) => {}",
                        arm.feature
                            .as_deref()
                            .map(|f| format!("{} ", rust_feature_cfg(f)))
                            .unwrap_or_default(),
                        filters::to_pascal_case(arm.body_alias.clone()),
                        arm.value & mask
                    )
                })
                .collect();
            // A catch-all arm presents the tag it could not name, and that is
            // what the carrier holds.
            if import.codec_variant_has_catch_all {
                arms.push(format!(
                    "{enum_path}::Default {{ tag, .. }} => {ty}::from(tag.get()) & 0x{mask:X}"
                ));
            }
            let accessor = self.accessor(embed);
            let arms = arms.join(", ");
            // The embedded body's source is read through its trait by path: the
            // trait is not imported here.
            let body_of = format!(
                "super::{}::{}Source::body",
                import.namespace, import.type_name
            );
            terms.push(if embed.present_if.is_some() {
                format!(
                    "match src.{accessor}() {{ Some(_x) => match {body_of}(_x) {{ {arms} }}, None => 0 }}"
                )
            } else {
                format!("match {body_of}(src.{accessor}()) {{ {arms} }}")
            });
        }
        lines.push(format!("let {local}: {ty} = {};", terms.join(" | ")));
        derived
            .flags
            .entry(field.clone())
            .or_default()
            .push(DerivedFlag {
                flag: flag.clone(),
                value: local,
                ty,
                kind: FlagValue::Number,
            });
    }

    /// The tag the active arm names, which the encode writes where the codec
    /// holds it. A default arm names none, so it presents the tag it holds.
    fn derive_variant_tag(&self, id: &ElementId, lines: &mut Vec<String>, derived: &mut Derived) {
        let enum_name = self.body_enum();
        let ty = self.l().type_name(&self.tag_type()).to_string();
        let local = self.local("_d_", id);
        let mut arms: Vec<String> = Vec::new();
        for arm in self.arms() {
            if let Some(feature) = &arm.feature {
                arms.push(format!("    {}", rust_feature_cfg(feature)));
            }
            match arm.value {
                Some(value) => arms.push(format!(
                    "    {enum_name}::{}(_) => {value},",
                    arm.variant_name
                )),
                None => arms.push(format!(
                    "    {enum_name}::Default {{ tag, .. }} => tag.get(),"
                )),
            }
        }
        lines.push(format!(
            "let {local}: {ty} = match src.body() {{\n{}\n}};",
            arms.join("\n")
        ));
        match id {
            ElementId::Field(name) => {
                derived.fields.insert(name.clone(), local);
            }
            ElementId::Flag { field, flag } => {
                derived
                    .flags
                    .entry(field.clone())
                    .or_default()
                    .push(DerivedFlag {
                        flag: flag.clone(),
                        value: local,
                        ty,
                        kind: FlagValue::Number,
                    });
            }
            ElementId::Variant => unreachable!("a tag is not the variant"),
        }
    }

    // ── derived values ───────────────────────────────────────────────

    /// The prologue: each derived element's value, computed and checked before
    /// anything is written. Returns the expression each element is read as.
    fn derivations(&self, lines: &mut Vec<String>) -> Derived {
        let mut derived = Derived::default();
        // A carrier a parent may set a flag of is read as a local, folded below.
        for (carrier, _) in self.overridable() {
            derived.flags.entry(carrier.id.clone()).or_default();
        }
        self.count_lists(lines);
        for (id, by) in self.roles.derived() {
            for derivation in by {
                match derivation {
                    Derivation::LengthOf { contents } => {
                        self.derive_length(id, contents, lines, &mut derived)
                    }
                    Derivation::CountOf { lists } => {
                        self.derive_count(id, lists, lines, &mut derived)
                    }
                    Derivation::EncodedLengthOf { bodies } => {
                        self.derive_encoded_length(id, bodies, lines, &mut derived)
                    }
                    Derivation::VariantTag { .. } => {
                        self.derive_variant_tag(id, lines, &mut derived)
                    }
                    Derivation::ParentTag { .. } => self.derive_parent_tag(id, lines, &mut derived),
                    Derivation::PresenceOf { gated } => {
                        self.derive_presence(id, gated, lines, &mut derived)
                    }
                    Derivation::BoundInput { binders } => {
                        self.derive_bound_input(id, binders, lines, &mut derived)
                    }
                    // The end flag of a chain is the entries' to set, as each is
                    // written (`streamed`), and no value of this codec's own.
                    Derivation::ChainContinuation { .. } => {}
                }
            }
        }
        derived.close(self, lines);
        derived
    }

    /// The expression for the number of bytes `field` holds, 0 when it is
    /// absent.
    fn byte_count(&self, field: &CodecField) -> String {
        let accessor = self.accessor(field);
        if field.present_if.is_some() {
            format!("src.{accessor}().map_or(0, |_v| _v.len())")
        } else {
            format!("src.{accessor}().len()")
        }
    }

    fn derive_length(
        &self,
        id: &ElementId,
        contents: &[codec_roles::Measured],
        lines: &mut Vec<String>,
        derived: &mut Derived,
    ) {
        let local = self.local("_d_", id);
        // Every field that names this element says its number; they agree or
        // there is no frame that says both.
        let mut numbers: Vec<Measure> = Vec::new();
        for (index, content) in contents.iter().enumerate() {
            let Some(field) = self.m().fields.iter().find(|f| f.id == content.field) else {
                continue;
            };
            let number = format!("_n{local}_{index}");
            // The wire holds `count - arith`: a number that cannot be negative.
            // A length plus a constant cannot pass `usize`, since a slice is
            // at most `isize::MAX` bytes.
            let measure = |count: &str| match content.arith {
                0 => count.to_string(),
                arith if arith > 0 => format!(
                    "{count}.checked_sub({arith}).ok_or({})?",
                    self.refuse(RefusalKind::LengthOutOfRange, &content.field)
                ),
                arith => format!("{count} + {}", -arith),
            };
            // A field gated off has no content to measure, so its length is the
            // wire's zero and not the arithmetic of an empty field.
            let value = if field.present_if.is_some() {
                let accessor = self.accessor(field);
                format!(
                    "match src.{accessor}() {{\n    Some(_v) => {},\n    None => 0,\n}}",
                    measure("_v.len()")
                )
            } else {
                measure(&self.byte_count(field))
            };
            lines.push(format!("let {number}: usize = {value};"));
            numbers.push(Measure {
                number,
                present: self.presence_test(field),
            });
        }
        self.derive_number(id, &local, numbers, lines, derived);
    }

    /// The test that `field` is there, for a field that may not be.
    fn presence_test(&self, field: &CodecField) -> Option<String> {
        field
            .present_if
            .is_some()
            .then(|| format!("src.{}().is_some()", self.accessor(field)))
    }

    /// The local that holds how many entries the list `field` holds.
    fn count_local(&self, field: &CodecField) -> String {
        format!("_k_{}", self.accessor(field))
    }

    /// Each list's entry count, as its source counted it. The source's type holds
    /// no more entries than the list's bound (docs/adr/0065 §6 R2), so the count
    /// is not checked here and no list is refused.
    fn count_lists(&self, lines: &mut Vec<String>) {
        for field in &self.m().fields {
            if !(field.is_repeat() || field.is_tlv_chain()) {
                continue;
            }
            let accessor = self.accessor(field);
            let count = if field.present_if.is_some() {
                format!("src.{accessor}().map_or(0, |_l| _l.len())")
            } else {
                format!("src.{accessor}().len()")
            };
            let local = self.count_local(field);
            lines.push(format!("let {local}: usize = {count};"));
        }
    }

    fn derive_count(
        &self,
        id: &ElementId,
        lists: &[String],
        lines: &mut Vec<String>,
        derived: &mut Derived,
    ) {
        let local = self.local("_d_", id);
        // The lists that name this element agree in size or no frame says both.
        let numbers: Vec<Measure> = lists
            .iter()
            .filter_map(|list| self.m().fields.iter().find(|f| &f.id == list))
            .map(|field| Measure {
                number: self.count_local(field),
                present: self.presence_test(field),
            })
            .collect();
        self.derive_number(id, &local, numbers, lines, derived);
    }

    /// The element `id` holds how many bytes the embedded `bodies` encode to:
    /// the body written into a sink that counts, which is the length the body's
    /// own encode gives it and cannot differ from it.
    fn derive_encoded_length(
        &self,
        id: &ElementId,
        bodies: &[String],
        lines: &mut Vec<String>,
        derived: &mut Derived,
    ) {
        let local = self.local("_d_", id);
        let mut numbers: Vec<Measure> = Vec::new();
        for body in bodies {
            let Some(field) = self.m().fields.iter().find(|f| &f.id == body) else {
                continue;
            };
            let child = self
                .child(field)
                .expect("`check` found the codec a body holds");
            let accessor = self.accessor(field);
            let size = format!("_s_{accessor}");
            let args = child_alias(field)
                .map(|alias| self.thread_args(alias, derived))
                .unwrap_or_default();
            // The body's own count ([`count_function`]): with nothing to handle
            // for a body that cannot refuse, and its refusal passed on for one
            // that can.
            let alias = child_alias(field).unwrap_or_default();
            let count = format!(
                "{}::count_source(_v{args}){}",
                child.type_name,
                if self.child_may_fail(alias) {
                    self.child_write(alias, &field.id)
                } else {
                    ""
                }
            );
            lines.push(if field.present_if.is_some() {
                format!(
                    "let {size}: usize = match src.{accessor}() {{\n    Some(_v) => {count},\n    None => 0,\n}};"
                )
            } else {
                format!("let {size}: usize = {{\n    let _v = src.{accessor}();\n    {count}\n}};")
            });
            // The length of a body that is not there is the wire's zero.
            numbers.push(Measure {
                number: size,
                present: None,
            });
        }
        self.derive_number(id, &local, numbers, lines, derived);
    }

    /// The derived number `n`, a `usize`, as the type `ty` of the field
    /// `element` it is written into (docs/adr/0065 §5): checked where the
    /// totality analysis names a refusal there, widened where the field is 64
    /// bits, and converted with nothing to check where the number's bound (the
    /// `max-count` its list was held to above) fits the field.
    fn narrow_number(&self, element: &str, ty: &SceType, n: &str) -> String {
        let rust = self.l().type_name(ty);
        if self
            .totality
            .refuses(RefusalKind::LengthOutOfRange, element)
        {
            format!(
                "{rust}::try_from({n}).map_err(|_| {})?",
                self.refuse(RefusalKind::LengthOutOfRange, element)
            )
        } else if *ty == SceType::Uint64 {
            format!("::sce_forge_runtime::codec::widen_usize({n})")
        } else {
            format!("{n} as {rust}")
        }
    }

    /// The element `id` holds the number the `measures` say.
    ///
    /// What is always there fixes the number, and anything that is there beside
    /// it says the same or there is no frame that says both. When all that is
    /// measured may be absent, the number is derived while some of it is there;
    /// when none is, the wire still holds a number, and the source states it
    /// ([`codec_totality::stated_when_absent`]).
    fn derive_number(
        &self,
        id: &ElementId,
        local: &str,
        measures: Vec<Measure>,
        lines: &mut Vec<String>,
        derived: &mut Derived,
    ) {
        let element = id.to_string();
        let disagree = || {
            format!(
                "{{\n    return Err({});\n}}",
                self.refuse(RefusalKind::InconsistentSource, &element)
            )
        };
        let always: Vec<&Measure> = measures.iter().filter(|m| m.present.is_none()).collect();
        let gated: Vec<&Measure> = measures.iter().filter(|m| m.present.is_some()).collect();
        // The number, as an expression of a `usize`, or the local that holds an
        // `Option<usize>` of it when there may be none to measure.
        let mut free: Option<String> = None;
        let mut number = "0usize".to_string();
        if let Some(anchor) = always.first() {
            for other in &always[1..] {
                lines.push(format!(
                    "if {} != {} {}",
                    other.number,
                    anchor.number,
                    disagree()
                ));
            }
            for g in &gated {
                lines.push(format!(
                    "if {} && {} != {} {}",
                    g.present.as_deref().unwrap_or("false"),
                    g.number,
                    anchor.number,
                    disagree()
                ));
            }
            number = anchor.number.clone();
        } else if !gated.is_empty() {
            let one = format!("_f{local}");
            let mut chain = String::new();
            for g in &gated {
                chain.push_str(&format!(
                    "if {} {{ Some({}) }} else ",
                    g.present.as_deref().unwrap_or("false"),
                    g.number
                ));
            }
            lines.push(format!("let {one}: Option<usize> = {chain}{{ None }};"));
            for g in &gated[1..] {
                lines.push(format!(
                    "if {} && Some({}) != {one} {}",
                    g.present.as_deref().unwrap_or("false"),
                    g.number,
                    disagree()
                ));
            }
            free = Some(one);
        }
        match id {
            ElementId::Field(name) => {
                let field = self
                    .m()
                    .fields
                    .iter()
                    .find(|f| &f.id == name)
                    .expect("a derived field is a field of the codec");
                let ty = self.l().type_name(&field.sce_type);
                let narrow = |n: &str| self.narrow_number(&element, &field.sce_type, n);
                match &free {
                    None => lines.push(format!("let {local}: {ty} = {};", narrow(&number))),
                    // Gated beside its content, it is written only where there
                    // is content, and then the content says it.
                    Some(one) if !self.stated(field) => lines.push(format!(
                        "let {local}: {ty} = {};",
                        narrow(&format!("{one}.unwrap_or(0)"))
                    )),
                    // Derived while there is content, and what the source states
                    // while there is not; a source that states a number beside
                    // content says it twice and must say the same.
                    Some(one) => {
                        let accessor = self.accessor(field);
                        lines.push(format!(
                            "let {local}: {ty} = match {one} {{\n    Some(_n) => {{\n        let _d = {};\n        if src.{accessor}().is_some_and(|_s| _s != _d) {{\n            return Err({});\n        }}\n        _d\n    }}\n    None => src.{accessor}().unwrap_or_default(),\n}};",
                            narrow("_n"),
                            self.refuse(RefusalKind::InconsistentSource, &element),
                        ));
                    }
                }
                derived.fields.insert(name.clone(), local.to_string());
            }
            ElementId::Flag { field, flag } => {
                // With none of it there, the carrier keeps the bits the source
                // presents.
                let (value, kind) = match free {
                    Some(one) => (one, FlagValue::Optional),
                    None => (number, FlagValue::Number),
                };
                derived
                    .flags
                    .entry(field.clone())
                    .or_default()
                    .push(DerivedFlag {
                        flag: flag.clone(),
                        value,
                        ty: "usize".to_string(),
                        kind,
                    });
            }
            ElementId::Variant => unreachable!("a length or a count is not the variant"),
        }
    }

    /// A flag bound to an input the held codecs say (docs/adr/0065 §6 R4a): the
    /// input of the body, or of the arm the value holds. Where no body or arm
    /// that binds it is there, the carrier keeps the bit the value presents.
    fn derive_bound_input(
        &self,
        id: &ElementId,
        binders: &[Binder],
        lines: &mut Vec<String>,
        derived: &mut Derived,
    ) {
        let local = self.local("_d_", id);
        let mut measures: Vec<Measure> = Vec::new();
        // Each arm that binds it: its pattern, with its `#[cfg]`, and what it says.
        let mut arms: Vec<(String, String)> = Vec::new();
        let says = |binder: &Binder, body: &str| {
            let child = self
                .child_of(&binder.alias)
                .expect("`check` found the codec a bind names");
            format!(
                "usize::from({}::input_{}({body}))",
                child.type_name,
                filters::to_snake_case(binder.input.clone())
            )
        };
        for (index, binder) in binders.iter().enumerate() {
            let number = format!("_n{local}_{index}");
            match &binder.site {
                BindSite::Embed { field } => {
                    let body = self
                        .m()
                        .fields
                        .iter()
                        .find(|f| &f.id == field)
                        .expect("a bind site is a field of the codec");
                    let accessor = self.accessor(body);
                    if body.present_if.is_some() {
                        lines.push(format!(
                            "let {number}: usize = src.{accessor}().map_or(0, |_b| {});",
                            says(binder, "_b")
                        ));
                        measures.push(Measure {
                            number,
                            present: Some(format!("src.{accessor}().is_some()")),
                        });
                    } else {
                        lines.push(format!(
                            "let {number}: usize = {};",
                            says(binder, &format!("src.{accessor}()"))
                        ));
                        measures.push(Measure {
                            number,
                            present: None,
                        });
                    }
                }
                BindSite::Arm => {
                    let enum_name = self.body_enum();
                    for arm in self.arms().iter().filter(|a| a.alias == binder.alias) {
                        let pattern = if arm.value.is_some() {
                            format!("{enum_name}::{}(_b)", arm.variant_name)
                        } else {
                            format!("{enum_name}::Default {{ body: _b, .. }}")
                        };
                        arms.push((format!("{}{pattern}", arm.cfg_prefix()), says(binder, "_b")));
                    }
                }
                BindSite::List { .. } => {
                    unreachable!("a list's bound flag is checked, not derived")
                }
            }
        }
        // The arms of the variant are one source, there when the value holds
        // one of them.
        if !arms.is_empty() && arms.len() == self.arms().len() {
            // Every arm binds it: the value always says it.
            let number = format!("_n{local}_arm");
            let arms: Vec<String> = arms
                .iter()
                .map(|(pattern, value)| format!("    {pattern} => {value},"))
                .collect();
            lines.push(format!(
                "let {number}: usize = match src.body() {{\n{}\n}};",
                arms.join("\n")
            ));
            measures.push(Measure {
                number,
                present: None,
            });
        } else if !arms.is_empty() {
            // One match says both whether an arm that binds it is there and what
            // it says. An arm under a build feature carries its `#[cfg]`, which
            // an alternative of `matches!` cannot.
            let said = format!("_o{local}_arm");
            let arms: Vec<String> = arms
                .iter()
                .map(|(pattern, value)| format!("    {pattern} => Some({value}),"))
                .collect();
            lines.push(format!(
                "let {said}: Option<usize> = match src.body() {{\n{}\n    _ => None,\n}};",
                arms.join("\n")
            ));
            measures.push(Measure {
                number: format!("{said}.unwrap_or(0)"),
                present: Some(format!("{said}.is_some()")),
            });
        }
        self.derive_number(id, &local, measures, lines, derived);
    }

    fn derive_presence(
        &self,
        id: &ElementId,
        gated: &[codec_roles::Gated],
        lines: &mut Vec<String>,
        derived: &mut Derived,
    ) {
        let ElementId::Flag { field, flag } = id else {
            unreachable!("a gate flag is a flag of a carrier");
        };
        let mut presences: Vec<String> = Vec::new();
        for (index, gate) in gated.iter().enumerate() {
            let Some(gated_field) = self.m().fields.iter().find(|f| f.id == gate.field) else {
                continue;
            };
            let present = format!("src.{}().is_some()", self.accessor(gated_field));
            // The flag is set exactly when the field is present, or exactly when
            // it is absent for a field gated on `!flag`.
            let local = format!("{}_{index}", self.local("_p_", id));
            lines.push(format!(
                "let {local}: bool = {};",
                if gate.when_clear {
                    format!("!{present}")
                } else {
                    present
                }
            ));
            presences.push(local);
        }
        for pair in presences.windows(2) {
            lines.push(format!(
                "if {} != {} {{\n    return Err({});\n}}",
                pair[0],
                pair[1],
                self.refuse(RefusalKind::InconsistentSource, &id.to_string()),
            ));
        }
        derived
            .flags
            .entry(field.clone())
            .or_default()
            .push(DerivedFlag {
                flag: flag.clone(),
                value: presences
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "false".to_string()),
                ty: "bool".to_string(),
                kind: FlagValue::Bool,
            });
    }

    // ── reading a field ──────────────────────────────────────────────

    /// The expression a field is read as: what the source presents, or what the
    /// encode derived for it.
    fn read(&self, field: &CodecField, derived: &Derived) -> String {
        if let Some(local) = derived.fields.get(&field.id) {
            return local.clone();
        }
        if derived.flags.contains_key(&field.id) {
            return self.carrier_local(field);
        }
        format!("src.{}()", self.accessor(field))
    }

    /// `expr` as the carrier the wire holds: an enum goes out as its
    /// underlying value.
    fn carrier(&self, field: &CodecField, expr: &str) -> String {
        self.l().codec_carrier_expr(&field.sce_type, expr)
    }

    // ── the wire: positional layout ──────────────────────────────────

    fn positional(&self, derived: &Derived, lines: &mut Vec<String>) {
        let m = self.m();
        let mut groups: std::collections::BTreeMap<u32, Vec<&CodecField>> =
            std::collections::BTreeMap::new();
        for field in &m.fields {
            if !field.is_variable_length() {
                groups.entry(field.byte_offset).or_default().push(field);
            }
        }
        for group in groups.values() {
            if let [field] = group.as_slice() {
                self.positional_single(field, derived, lines);
            } else {
                let parts: Vec<String> = group
                    .iter()
                    .map(|field| {
                        let bit_off = field.bit_offset.unwrap_or(0);
                        let bits = field.fixed_bits().unwrap_or(8);
                        let mask = (1u64 << bits) - 1;
                        let value = self.carrier(field, &self.read(field, derived));
                        // Drop the `<< 0` no-op for a bit-offset-0 field.
                        if bit_off == 0 {
                            format!("({value} & 0x{mask:02X})")
                        } else {
                            format!("(({value} & 0x{mask:02X}) << {bit_off})")
                        }
                    })
                    .collect();
                lines.push(format!("w.write_u8({})?;", parts.join(" | ")));
            }
        }
        for field in m.fields.iter().filter(|f| f.is_variable_length()) {
            if m.has_dma_aligned_fields() {
                lines.push(format!(
                    "while w.position() < {} {{\n    w.write_u8(0)?;\n}}",
                    field.byte_offset
                ));
            }
            let value = self.read(field, derived);
            lines.push(format!("w.write_bytes({})?;", self.bytes_of(field, &value)));
        }
    }

    fn positional_single(&self, field: &CodecField, derived: &Derived, lines: &mut Vec<String>) {
        let value = self.carrier(field, &self.read(field, derived));
        let bit_off = field.bit_offset.unwrap_or(0);
        let endian = field.effective_endian(self.m().default_endian);
        match field.fixed_bits() {
            Some(8) if bit_off == 0 => lines.push(format!("w.write_u8({value})?;")),
            Some(bits) if bits < 8 || bit_off > 0 => {
                let mask = (1u64 << bits) - 1;
                // Drop the `<< 0` no-op for a bit-offset-0 field.
                let inner = if bit_off == 0 {
                    format!("{value} & 0x{mask:02X}")
                } else {
                    format!("({value} & 0x{mask:02X}) << {bit_off}")
                };
                lines.push(format!("w.write_u8({inner})?;"));
            }
            Some(byte_count @ (16 | 24 | 32)) => {
                for e in encode_multibyte_unified(
                    &value,
                    byte_count / 8,
                    endian,
                    crate::generator::Language::Rust,
                ) {
                    lines.push(format!("w.write_u8({e})?;"));
                }
            }
            // No other width has a positional form; the codec's decode does not
            // read one either.
            _ => {}
        }
    }

    // ── the wire: streamed layout ────────────────────────────────────

    fn streamed(&self, field: &CodecField, derived: &Derived, lines: &mut Vec<String>) {
        let gated = field.present_if.is_some();
        // A field derived whole whose gate is its own is written where it is
        // present: the gate flag is derived from the field it gates, and a length
        // field that is itself gated is announced by the same flag.
        let value = if gated && self.presented(field) {
            "_v".to_string()
        } else {
            self.carrier_read(field, derived)
        };
        let mut body: Vec<String> = Vec::new();
        match &field.bit_size {
            BitSize::Fixed { bits } => {
                let n = bits.div_ceil(8);
                let endian = field.effective_endian(self.m().default_endian);
                let source = self.carrier(field, &value);
                if n == 1 {
                    body.push(format!("w.write_u8({source})?;"));
                } else {
                    for e in encode_multibyte_unified(
                        &source,
                        n,
                        endian,
                        crate::generator::Language::Rust,
                    ) {
                        body.push(format!("w.write_u8({e})?;"));
                    }
                }
            }
            BitSize::Tail | BitSize::LengthRef => {
                body.push(format!("w.write_bytes({})?;", self.bytes_of(field, &value)));
            }
            BitSize::Vle { width_bits } => {
                body.push(format!("w.write_vle_u{width_bits}({value})?;"));
            }
            BitSize::Repeat { .. } | BitSize::TlvChain { .. } => {
                let child = self
                    .child(field)
                    .expect("`check` found the codec a list holds");
                // The inputs of the entry codec that the chain supplies: one
                // value for each entry, 1 when the entry just written has the
                // identifier the bind waits for, and 0 for the first.
                // An input the entry says is checked in the prologue and not
                // handed to it (docs/adr/0065 §6 R4a).
                let alias = child_alias(field).unwrap_or_default();
                let binds: Vec<(String, String, u64)> = match &field.bit_size {
                    BitSize::TlvChain { entry_binds, .. } => entry_binds
                        .iter()
                        .filter(|bind| !self.child_says_input(alias, &bind.input))
                        .map(|bind| {
                            (
                                bind.input.clone(),
                                format!("_prev_{}", filters::to_snake_case(bind.input.clone())),
                                bind.previous_entry_id,
                            )
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                for (_, local, _) in &binds {
                    body.push(format!("let mut {local}: u8 = 0;"));
                }
                let supplied = |name: &str| {
                    binds
                        .iter()
                        .find(|(input, _, _)| input == name)
                        .map(|(_, local, _)| local.clone())
                };
                let args = child_alias(field)
                    .map(|alias| self.thread_args_supplying(alias, derived, &supplied))
                    .unwrap_or_default();
                let pass = self.child_write(child_alias(field).unwrap_or_default(), &field.id);
                let updates: Vec<String> = binds
                    .iter()
                    .map(|(_, local, previous)| {
                        format!(
                            "    {local} = u8::from({} == {previous}u64);",
                            self.entry_identifier(field, "&_e")
                        )
                    })
                    .collect();
                match self.chain_end_flag(field) {
                    // Every entry but the last says another follows: the flag is
                    // derived from where the entry stands, and set as it is
                    // written.
                    Some(member) => {
                        let count = self.count_local(field);
                        body.push(format!("for (_i, _e) in {value}.enumerate() {{"));
                        body.push(format!(
                            "    {}::write_source_in(&_e, w{args}, &{}::{member}(_i + 1 < {count})){pass};",
                            child.type_name, child.overrides_path
                        ));
                    }
                    None => {
                        body.push(format!("for _e in {value} {{"));
                        body.push(format!(
                            "    {}::write_source(&_e, w{args}){pass};",
                            child.type_name
                        ));
                    }
                }
                body.extend(updates);
                body.push("}".to_string());
            }
            BitSize::Embed => {
                let child = self
                    .child(field)
                    .expect("`check` found the codec a body holds");
                let args = child_alias(field)
                    .map(|alias| self.thread_args(alias, derived))
                    .unwrap_or_default();
                body.push(format!(
                    "{}::write_source({value}, w{args}){};",
                    child.type_name,
                    self.child_write(child_alias(field).unwrap_or_default(), &field.id)
                ));
            }
        }
        if gated && self.presented(field) {
            let accessor = self.accessor(field);
            lines.push(format!("if let Some(_v) = src.{accessor}() {{"));
            lines.extend(body.into_iter().map(|line| format!("    {line}")));
            lines.push("}".to_string());
        } else if gated {
            // Derived whole and gated: written where the field it gates is
            // present is the gate's business, and this field's gate is one flag
            // of a carrier, read as the codec reads it.
            lines.extend(self.guarded_by_flag(field, body, derived));
        } else {
            lines.extend(body);
        }
    }

    /// A derived field that is itself gated: written when its gate flag is set,
    /// as the gate flag the codec computes says.
    fn guarded_by_flag(
        &self,
        field: &CodecField,
        body: Vec<String>,
        derived: &Derived,
    ) -> Vec<String> {
        let gate = field.present_if.as_ref().expect("a gated field has a gate");
        let mut out = vec![format!("if {} {{", self.gate_test(gate, derived))];
        out.extend(body.into_iter().map(|line| format!("    {line}")));
        out.push("}".to_string());
        out
    }

    fn carrier_read(&self, field: &CodecField, derived: &Derived) -> String {
        self.read(field, derived)
    }

    /// The `&[u8]` a bytes or text value is written as.
    fn bytes_of(&self, field: &CodecField, value: &str) -> String {
        if field.is_string() {
            format!("{value}.as_bytes()")
        } else {
            value.to_string()
        }
    }
}

/// What the prologue computed: the value of each derived element.
#[derive(Default)]
struct Derived {
    /// Fields derived whole, by id: the local that holds the value.
    fields: std::collections::BTreeMap<String, String>,
    /// Flags of a carrier that are derived, by carrier id. The carrier's value
    /// is the one its source presents, with those bit-ranges replaced.
    flags: std::collections::BTreeMap<String, Vec<DerivedFlag>>,
}

struct DerivedFlag {
    flag: String,
    /// A local that holds the number (`Number`) or the truth (`Bool`).
    value: String,
    /// The Rust type of that local, so that a conversion to the carrier's type
    /// is written only where the two differ.
    ty: String,
    kind: FlagValue,
}

enum FlagValue {
    Number,
    Bool,
    /// A number there may be none of: the carrier keeps the bits the source
    /// presents when there is not.
    Optional,
}

/// What a derived number is measured from: the local that holds the measure (0
/// while the content is absent), and, for content that may be absent, the test
/// that it is there.
struct Measure {
    number: String,
    present: Option<String>,
}

impl Derived {
    /// Fold the derived flags of each carrier into the carrier's value, which
    /// the wire then reads as `_c_<carrier>`. Emitted after the flags' values
    /// exist and before the first write.
    fn close(&self, plan: &Plan<'_>, lines: &mut Vec<String>) {
        for (carrier_id, flags) in &self.flags {
            let carrier = plan
                .m()
                .fields
                .iter()
                .find(|f| &f.id == carrier_id)
                .expect("a derived flag belongs to a carrier of the codec");
            let ty = plan.l().type_name(&carrier.sce_type);
            // The carrier as the source presents it, each derived bit-range
            // replaced in turn. `compound` is whether `value` needs parentheses
            // to be the left operand of the next `&`.
            let mut value = format!("src.{}()", plan.accessor(carrier));
            let mut compound = false;
            // Flags that are a number only while there is content to measure.
            let mut optional: Vec<(&DerivedFlag, &FlagDef)> = Vec::new();
            for derived in flags {
                let def = carrier
                    .flags
                    .iter()
                    .find(|d| d.name == derived.flag)
                    .expect("a derived flag is a flag of its carrier");
                let width_mask = if def.width >= 64 {
                    u64::MAX
                } else {
                    (1u64 << def.width) - 1
                };
                let mask = width_mask << def.bit;
                let element = format!("{carrier_id}.{}", derived.flag);
                let bits = match derived.kind {
                    FlagValue::Optional => {
                        optional.push((derived, def));
                        continue;
                    }
                    FlagValue::Bool => format!("{ty}::from({})", derived.value),
                    FlagValue::Number => {
                        // A number the flag's bit-range cannot hold is not a
                        // frame. Where the grammar bounds it within the bits (a
                        // known arm's tag, a parent's tag masked to them, a
                        // count held to its list's bound), there is nothing to
                        // check.
                        if plan
                            .totality
                            .refuses(RefusalKind::LengthOutOfRange, &element)
                        {
                            lines.push(format!(
                                "if {} > 0x{width_mask:X} {{\n    return Err({});\n}}",
                                derived.value,
                                plan.refuse(RefusalKind::LengthOutOfRange, &element)
                            ));
                        }
                        if derived.ty == ty {
                            // Already the carrier's type: a conversion to it
                            // is the identity.
                            derived.value.clone()
                        } else {
                            // Within the bit-range, so within the carrier's
                            // type: the conversion keeps the value.
                            let narrowed = format!(
                                "_v_{}_{}",
                                plan.l().codec_field_id(&carrier.id),
                                filters::to_snake_case(derived.flag.clone())
                            );
                            lines
                                .push(format!("let {narrowed}: {ty} = {} as {ty};", derived.value));
                            narrowed
                        }
                    }
                };
                // Drop the `<< 0` no-op for a flag at bit 0.
                let placed = if def.bit == 0 {
                    bits
                } else {
                    format!("({bits} << {})", def.bit)
                };
                let kept = if compound {
                    format!("({value})")
                } else {
                    value
                };
                value = format!("({kept} & !0x{mask:X}) | {placed}");
                compound = true;
            }
            // The flags a parent sets replace the bits the source presents.
            let sets: Vec<&FlagDef> = plan
                .overridable()
                .into_iter()
                .filter(|(c, _)| c.id == carrier.id)
                .map(|(_, def)| def)
                .collect();
            let local = plan.carrier_local(carrier);
            let binding = if sets.is_empty() && optional.is_empty() {
                "let"
            } else {
                "let mut"
            };
            lines.push(format!("{binding} {local}: {ty} = {value};"));
            for (derived, def) in optional {
                let width_mask = if def.width >= 64 {
                    u64::MAX
                } else {
                    (1u64 << def.width) - 1
                };
                let element = format!("{carrier_id}.{}", derived.flag);
                // Within the bit-range, so within the carrier's type.
                let placed = shl(&format!("(_n as {ty})"), def.bit);
                let check = if plan
                    .totality
                    .refuses(RefusalKind::LengthOutOfRange, &element)
                {
                    format!(
                        "    if _n > 0x{width_mask:X} {{\n        return Err({});\n    }}\n",
                        plan.refuse(RefusalKind::LengthOutOfRange, &element)
                    )
                } else {
                    String::new()
                };
                lines.push(format!(
                    "if let Some(_n) = {} {{\n{check}    {local} = ({local} & !0x{mask:X}) | {placed};\n}}",
                    derived.value,
                    mask = width_mask << def.bit,
                ));
            }
            for def in sets {
                lines.push(format!(
                    "if let Some(_o) = over.{member} {{\n    {local} = ({local} & !0x{mask:X}) | {placed};\n}}",
                    member = plan.override_member(carrier, &def.name),
                    mask = 1u64 << def.bit,
                    placed = shl(&format!("{ty}::from(_o)"), def.bit),
                ));
            }
        }
    }
}

/// `value` shifted right by `bit`, with no shift at bit 0 (a shift by nothing is
/// an operation clippy refuses in generated code).
fn shr(value: &str, bit: u32) -> String {
    if bit == 0 {
        value.to_string()
    } else {
        format!("({value} >> {bit})")
    }
}

/// `value` shifted left by `bit`, with no shift at bit 0.
fn shl(value: &str, bit: u32) -> String {
    if bit == 0 {
        value.to_string()
    } else {
        format!("({value} << {bit})")
    }
}

/// `lines` as a block at `columns` of indent, each statement on its own line.
fn indent(lines: &[String], columns: usize) -> String {
    let pad = " ".repeat(columns);
    let mut out = String::new();
    for line in lines {
        for part in line.lines() {
            out.push_str(&pad);
            out.push_str(part);
            out.push('\n');
        }
    }
    out
}
