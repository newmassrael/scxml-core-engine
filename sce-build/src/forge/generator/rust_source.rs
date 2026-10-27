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
use crate::forge::codec_roles::{self, CodecRoles, Derivation, ElementId, Role};

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
}

/// The source interface of a codec, as the text a codec file holds, or why the
/// codec has none yet.
pub(super) fn render(input: &Input<'_>) -> Result<String, Unsupported> {
    let plan = Plan::of(input)?;
    Ok(plan.text())
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
}

/// How one field is read and written.
struct Plan<'a> {
    input: &'a Input<'a>,
    roles: CodecRoles,
    /// `{Struct}Source`.
    trait_name: String,
}

/// Whether `m` has a source interface, given whether each codec it imports has
/// one. Shared by the codec's own generation and by the enrichment of a parent
/// that imports it, so the two cannot disagree on which codecs the interface
/// covers.
pub(crate) fn supported_with(
    m: &CodecModel,
    mut import_has_source: impl FnMut(&str) -> bool,
) -> bool {
    let roles = codec_roles::classify(m, &[]);
    // The answer for each codec the model holds, asked once and before the
    // check, which cannot take a closure that borrows its caller's walk.
    let known: std::collections::BTreeMap<&str, bool> = child_aliases(m)
        .into_iter()
        .map(|alias| (alias, import_has_source(alias)))
        .collect();
    check(m, &roles, &|alias| {
        known.get(alias).copied().unwrap_or(false)
    })
    .is_ok()
}

/// The shapes the interface covers.
fn check(
    m: &CodecModel,
    roles: &CodecRoles,
    import_has_source: &dyn Fn(&str) -> bool,
) -> Result<(), Unsupported> {
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
    for (id, by) in roles.derived() {
        for derivation in by {
            match derivation {
                Derivation::LengthOf { .. }
                | Derivation::PresenceOf { .. }
                | Derivation::CountOf { .. }
                | Derivation::EncodedLengthOf { .. }
                | Derivation::VariantTag { .. }
                | Derivation::ChainContinuation { .. } => {}
                other => return no(format!("`{id}` is derived by {other}")),
            }
        }
    }
    // A gate the codec cannot derive is checked against the source, except the
    // membership of a chain, which is not read yet.
    for field in &m.fields {
        let mut clause = field.present_if.as_ref();
        while let Some(p) = clause {
            if p.scope == PresentIfScope::Chain {
                return no(format!("`{}`: a gate on the entries of a chain", field.id));
            }
            clause = p.or_with.as_deref();
        }
        if let BitSize::TlvChain { entry_binds, .. } = &field.bit_size {
            if !entry_binds.is_empty() {
                return no(format!(
                    "`{}`: a flag-input supplied by the entry before",
                    field.id
                ));
            }
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

/// The import alias of the codec a list or an embedded body holds.
fn child_alias(field: &CodecField) -> Option<&str> {
    if field.is_repeat() {
        field.repeat_body_alias.as_deref()
    } else if field.is_tlv_chain() {
        field.tlv_chain_body_alias.as_deref()
    } else if field.is_embed() {
        field.embed_body_alias.as_deref()
    } else {
        None
    }
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
        check(input.m, &roles, &has_source)?;
        Ok(Plan {
            input,
            roles,
            trait_name: format!("{}Source", input.struct_name),
        })
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
        format!(
            "/// Flags of `{name}` a parent sets in place of the ones a value presents:\n\
             /// the end flag of a chain whose entries are `{name}`, which the chain\n\
             /// derives from where an entry stands.\n\
             #[doc(hidden)]\n\
             #[derive(Debug, Default, Clone, Copy)]\n\
             pub struct {overrides} {{\n{members}}}\n\n",
            name = self.name(),
            overrides = self.overrides_name(),
        )
    }

    /// Whether the codec holds a list, an embedded body or a variant: a refusal
    /// that comes from inside one has to be found before the first byte is
    /// written.
    fn has_children(&self) -> bool {
        !child_aliases(self.m()).is_empty()
    }

    // ── flag-inputs ──────────────────────────────────────────────────

    /// The parameter a flag-input is passed as.
    fn input_name(input: &FlagInput) -> String {
        filters::to_snake_case(input.name.clone())
    }

    /// `, n: u8` for each flag-input the codec takes from its parent: a bit of
    /// the parent's own header that gates a field here, so it is a fact the
    /// source cannot present.
    fn input_params(&self) -> String {
        self.m()
            .flag_inputs
            .iter()
            .map(|input| format!(", {}: u8", Self::input_name(input)))
            .collect()
    }

    /// `, n` for each flag-input, passed on.
    fn input_args(&self) -> String {
        self.m()
            .flag_inputs
            .iter()
            .map(|input| format!(", {}", Self::input_name(input)))
            .collect()
    }

    /// The arguments a parent passes the codec `alias`, one for each flag-input
    /// it takes, in the order it declares them: the bits of the parent's own
    /// carrier that the import binds, or the parent's own input passed on.
    fn thread_args(&self, alias: &str, derived: &Derived) -> String {
        let Some(import) = self.input.imports.iter().find(|i| i.alias == alias) else {
            return String::new();
        };
        let mut out = String::new();
        for input in &import.codec_flag_inputs {
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
                    let bits = format!("({read} >> {}) & 0x{mask:X}", def.bit);
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

    /// Each gate the codec cannot derive the presence from, held to the source:
    /// the field is present exactly when the gate says, or the value is not one
    /// frame can carry. Made in the prologue, so nothing is written first.
    fn gate_checks(&self, derived: &Derived, lines: &mut Vec<String>) {
        for check in &self.roles.checks {
            let Some(field) = self.m().fields.iter().find(|f| f.id == check.field) else {
                continue;
            };
            let Some(gate) = &field.present_if else {
                continue;
            };
            lines.push(format!(
                "if ({}) != src.{}().is_some() {{\n    return Err(CodecError::PresentIfMismatch);\n}}",
                self.gate_test(gate, derived),
                self.accessor(field),
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

    /// Whether a source presents `field`: it is not derived whole.
    fn presented(&self, field: &CodecField) -> bool {
        !matches!(
            self.roles.role_of(&ElementId::Field(field.id.clone())),
            Some(Role::Derived(_))
        )
    }

    /// The type an accessor returns for the value of `field` when it is
    /// present: a scalar by value, bytes and text by reference.
    fn value_type(&self, field: &CodecField) -> String {
        if let Some(child) = self.child(field) {
            // An embedded body is read by reference, a list as the entries it
            // holds in order: any iterator over values that say an entry.
            return if field.is_embed() {
                format!("&impl {}", child.trait_path)
            } else {
                format!("impl Iterator<Item = impl {}> + Clone", child.trait_path)
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
        if field.present_if.is_some() {
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
                });
            }
        }
        arms
    }

    /// The type a tag takes: its carrier's, or the narrowest unsigned type that
    /// holds the bit-range it is read from.
    fn tag_type(&self) -> SceType {
        let Some(variant) = &self.m().variant else {
            return SceType::Uint8;
        };
        let flag_def = match (&variant.peek_byte, &variant.tag_flag) {
            (Some(peek), Some(flag)) => peek.flags.iter().find(|f| &f.name == flag),
            (None, Some(flag)) => variant
                .tag_field
                .as_ref()
                .and_then(|id| self.m().fields.iter().find(|f| &f.id == id))
                .and_then(|carrier| carrier.flags.iter().find(|f| &f.name == flag)),
            _ => None,
        };
        match flag_def {
            Some(def) => match def.width.max(1) {
                0..=8 => SceType::Uint8,
                9..=16 => SceType::Uint16,
                17..=32 => SceType::Uint32,
                _ => SceType::Uint64,
            },
            None => variant
                .tag_field
                .as_ref()
                .and_then(|id| self.m().fields.iter().find(|f| &f.id == id))
                .map_or(SceType::Uint8, |carrier| carrier.sce_type.clone()),
        }
    }

    /// What the accessor of the variant returns: the enum, with the source of
    /// each arm's body as a type of its own.
    fn variant_type(&self) -> String {
        let args: Vec<String> = self
            .arms()
            .iter()
            .map(|arm| format!("impl {}", arm.child.trait_path))
            .collect();
        format!("{}<'_, {}>", self.body_enum(), args.join(", "))
    }

    fn body_enum_definition(&self) -> String {
        let arms = self.arms();
        if arms.is_empty() {
            return String::new();
        }
        let generics: Vec<String> = arms
            .iter()
            .map(|arm| format!("{}: ?Sized", arm.generic))
            .collect();
        let tag = self.l().type_name(&self.tag_type()).to_string();
        let mut variants = String::new();
        for arm in &arms {
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
             pub enum {enum_name}<'s, {generics}> {{\n{variants}}}\n\n",
            name = self.name(),
            enum_name = self.body_enum(),
            generics = generics.join(", "),
        )
    }

    fn text(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.body_enum_definition());
        out.push_str(&self.overrides_definition());
        out.push_str(&self.trait_definition());
        out.push_str(&self.reference_impl());
        out.push_str(&self.builder_impl());
        out.push_str(&self.encode_impl());
        out
    }

    fn trait_definition(&self) -> String {
        let mut items = String::new();
        for field in &self.m().fields {
            if !self.presented(field) {
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
            items.push_str(&format!(
                "    /// The arm of the variant this value holds.\n    fn body(&self) -> {};\n",
                self.variant_type()
            ));
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
            if !self.presented(field) {
                continue;
            }
            let accessor = self.accessor(field);
            items.push_str(&format!(
                "    fn {accessor}(&self) -> {ty} {{\n        (**self).{accessor}()\n    }}\n",
                ty = self.accessor_type(field),
            ));
        }
        if self.m().variant.is_some() {
            items.push_str(&format!(
                "    fn body(&self) -> {} {{\n        (**self).body()\n    }}\n",
                self.variant_type()
            ));
        }
        format!(
            "impl<T: {trait_name} + ?Sized> {trait_name} for &T {{\n{items}}}\n\n",
            trait_name = self.trait_name,
        )
    }

    /// The builder says what it holds.
    fn builder_impl(&self) -> String {
        let mut items = String::new();
        for field in &self.m().fields {
            if !self.presented(field) {
                continue;
            }
            let accessor = self.accessor(field);
            let gated = field.present_if.is_some();
            // A list or an embedded body is lent, a scalar copied or reborrowed.
            let read = if field.is_embed() {
                if gated {
                    format!("self.{accessor}.as_ref()")
                } else {
                    format!("&self.{accessor}")
                }
            } else if self.child(field).is_some() {
                if gated {
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
            // The arm the value holds, each body lent.
            let enum_name = self.body_enum();
            let variant = format!("{}Variant", self.name());
            let mut arms = String::new();
            for arm in self.arms() {
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
            items.push_str(&format!(
                "    fn body(&self) -> {ty} {{\n        match &self.body {{\n{arms}        }}\n    }}\n",
                ty = self.variant_type(),
            ));
        }
        format!(
            "impl{lt} {trait_name} for {name}{lt} {{\n{items}}}\n\n",
            lt = self.lt(),
            name = self.name(),
            trait_name = self.trait_name,
        )
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
        let guard = if self.has_children() {
            format!(
                "\x20       Self::write_source(src, &mut ::sce_forge_runtime::codec::CountSink::new(){args})?;\n"
            )
        } else {
            String::new()
        };
        format!(
            "#[allow(dead_code)]\n\
             impl{lt} {name}{lt} {{\n\
             \x20   /// Encode what `src` says into the caller-owned sink: the bytes\n\
             \x20   /// [`Self::encode`] writes for the value it holds, read through\n\
             \x20   /// [`{trait_name}`]. A length, a count or a gate flag is computed from what\n\
             \x20   /// `src` presents, and a value the encoding cannot carry is refused\n\
             \x20   /// before the first byte is written.\n\
             \x20   pub fn encode_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K{params}) -> Result<(), CodecError> {{\n\
             {guard}\
             \x20       Self::write_source(src, w{args})\n\
             \x20   }}\n\n\
             {write}\
             }}\n",
            lt = self.lt(),
            name = self.name(),
            trait_name = self.trait_name,
        )
    }

    /// `write_source`: the wire without the guard a parent has already made. A
    /// codec with flags a parent may set also has `write_source_in`, which
    /// takes them, and `write_source` is that with none set.
    fn write_functions(&self, body: &str) -> String {
        let trait_name = &self.trait_name;
        let params = self.input_params();
        let args = self.input_args();
        if self.overridable().is_empty() {
            return format!(
                "\x20   /// [`Self::encode_source`] without the guard a parent has already made.\n\
                 \x20   #[doc(hidden)]\n\
                 \x20   pub fn write_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K{params}) -> Result<(), CodecError> {{\n\
                 {body}\
                 \x20   }}\n"
            );
        }
        let overrides = self.overrides_name();
        format!(
            "\x20   /// [`Self::encode_source`] without the guard a parent has already made.\n\
             \x20   #[doc(hidden)]\n\
             \x20   pub fn write_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K{params}) -> Result<(), CodecError> {{\n\
             \x20       Self::write_source_in(src, w{args}, &{overrides}::default())\n\
             \x20   }}\n\n\
             \x20   /// [`Self::write_source`] with the flags a parent sets in place of the\n\
             \x20   /// ones `src` presents.\n\
             \x20   #[doc(hidden)]\n\
             \x20   pub fn write_source_in<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K{params}, over: &{overrides}) -> Result<(), CodecError> {{\n\
             {body}\
             \x20   }}\n"
        )
    }

    /// The statements of `write_source`: the derived values, then the wire.
    fn write_body(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        let derived = self.derivations(&mut lines);
        self.gate_checks(&derived, &mut lines);
        for input in &self.m().flag_inputs {
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
            lines.push(format!(
                "    {pattern} => {}::write_source(_b, w{})?,",
                arm.child.type_name,
                self.thread_args(&arm.alias, derived)
            ));
        }
        lines.push("}".to_string());
    }

    /// The tag the active arm names, which the encode writes where the codec
    /// holds it. A default arm names none, so it presents the tag it holds.
    fn derive_variant_tag(&self, id: &ElementId, lines: &mut Vec<String>, derived: &mut Derived) {
        let enum_name = self.body_enum();
        let ty = self.l().type_name(&self.tag_type()).to_string();
        let local = self.local("_d_", id);
        let mut arms: Vec<String> = Vec::new();
        for arm in self.arms() {
            match arm.value {
                Some(value) => arms.push(format!(
                    "    {enum_name}::{}(_) => {value},",
                    arm.variant_name
                )),
                None => arms.push(format!("    {enum_name}::Default {{ tag, .. }} => tag,")),
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
        self.bound_lists(lines);
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
                    Derivation::PresenceOf { gated } => {
                        self.derive_presence(id, gated, lines, &mut derived)
                    }
                    // The end flag of a chain is the entries' to set, as each is
                    // written (`streamed`), and no value of this codec's own.
                    Derivation::ChainContinuation { .. } => {}
                    _ => unreachable!("`check` refused every other derivation"),
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
        let mut numbers: Vec<String> = Vec::new();
        for (index, content) in contents.iter().enumerate() {
            let Some(field) = self.m().fields.iter().find(|f| f.id == content.field) else {
                continue;
            };
            let number = format!("_n{local}_{index}");
            // The wire holds `count - arith`: a number that cannot be negative or
            // larger than usize.
            let measure = |count: &str| match content.arith {
                0 => count.to_string(),
                arith if arith > 0 => {
                    format!("{count}.checked_sub({arith}).ok_or(CodecError::LengthOutOfRange)?")
                }
                arith => format!(
                    "{count}.checked_add({}).ok_or(CodecError::LengthOutOfRange)?",
                    -arith
                ),
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
            numbers.push(number);
        }
        self.derive_number(id, &local, numbers, lines, derived);
    }

    /// The local that holds how many entries the list `field` holds.
    fn count_local(&self, field: &CodecField) -> String {
        format!("_k_{}", self.accessor(field))
    }

    /// Each list's entry count, checked against the bound the list declares: a
    /// list the codec could not decode back is not a frame.
    fn bound_lists(&self, lines: &mut Vec<String>) {
        for field in &self.m().fields {
            if !(field.is_repeat() || field.is_tlv_chain()) {
                continue;
            }
            let accessor = self.accessor(field);
            let count = if field.present_if.is_some() {
                format!("src.{accessor}().map_or(0, |_l| _l.count())")
            } else {
                format!("src.{accessor}().count()")
            };
            let local = self.count_local(field);
            lines.push(format!("let {local}: usize = {count};"));
            let (bound, refusal) = match &field.bit_size {
                BitSize::TlvChain { max_depth, .. } => (*max_depth, "TlvChainOverflow"),
                _ => (
                    crate::forge::limits::resolve_max_count(field.max_count),
                    "TooManyElements",
                ),
            };
            lines.push(format!(
                "if {local} > {bound} {{\n    return Err(CodecError::{refusal});\n}}"
            ));
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
        let numbers: Vec<String> = lists
            .iter()
            .filter_map(|list| self.m().fields.iter().find(|f| &f.id == list))
            .map(|field| self.count_local(field))
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
        let mut numbers: Vec<String> = Vec::new();
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
            let write = format!("{}::write_source(_v, &mut _c{args})?;", child.type_name);
            let body_write = if field.present_if.is_some() {
                format!("if let Some(_v) = src.{accessor}() {{\n    {write}\n}}")
            } else {
                format!("let _v = src.{accessor}();\n{write}")
            };
            lines.push(format!(
                "let {size}: usize = {{\n    let mut _c = ::sce_forge_runtime::codec::CountSink::new();\n{}\n    _c.len()\n}};",
                indent(&[body_write], 4).trim_end_matches('\n')
            ));
            numbers.push(size);
        }
        self.derive_number(id, &local, numbers, lines, derived);
    }

    /// The element `id` holds the number the locals in `numbers` all say.
    fn derive_number(
        &self,
        id: &ElementId,
        local: &str,
        numbers: Vec<String>,
        lines: &mut Vec<String>,
        derived: &mut Derived,
    ) {
        for pair in numbers.windows(2) {
            lines.push(format!(
                "if {} != {} {{\n    return Err(CodecError::InconsistentSource);\n}}",
                pair[0], pair[1]
            ));
        }
        let number = numbers
            .first()
            .cloned()
            .unwrap_or_else(|| "0usize".to_string());
        match id {
            ElementId::Field(name) => {
                let field = self
                    .m()
                    .fields
                    .iter()
                    .find(|f| &f.id == name)
                    .expect("a derived field is a field of the codec");
                let ty = self.l().type_name(&field.sce_type);
                lines.push(format!(
                    "let {local}: {ty} = {ty}::try_from({number}).map_err(|_| CodecError::LengthOutOfRange)?;"
                ));
                derived.fields.insert(name.clone(), local.to_string());
            }
            ElementId::Flag { field, flag } => {
                derived
                    .flags
                    .entry(field.clone())
                    .or_default()
                    .push(DerivedFlag {
                        flag: flag.clone(),
                        value: number,
                        kind: FlagValue::Number,
                    });
            }
            ElementId::Variant => unreachable!("a length or a count is not the variant"),
        }
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
                "if {} != {} {{\n    return Err(CodecError::InconsistentSource);\n}}",
                pair[0], pair[1]
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
                let args = child_alias(field)
                    .map(|alias| self.thread_args(alias, derived))
                    .unwrap_or_default();
                match self.chain_end_flag(field) {
                    // Every entry but the last says another follows: the flag is
                    // derived from where the entry stands, and set as it is
                    // written.
                    Some(member) => {
                        let count = self.count_local(field);
                        body.push(format!("for (_i, _e) in {value}.enumerate() {{"));
                        body.push(format!(
                            "    {}::write_source_in(&_e, w{args}, &{} {{ {member}: Some(_i + 1 < {count}), ..Default::default() }})?;",
                            child.type_name, child.overrides_path
                        ));
                        body.push("}".to_string());
                    }
                    None => {
                        body.push(format!("for _e in {value} {{"));
                        body.push(format!(
                            "    {}::write_source(&_e, w{args})?;",
                            child.type_name
                        ));
                        body.push("}".to_string());
                    }
                }
            }
            BitSize::Embed => {
                let child = self
                    .child(field)
                    .expect("`check` found the codec a body holds");
                let args = child_alias(field)
                    .map(|alias| self.thread_args(alias, derived))
                    .unwrap_or_default();
                body.push(format!(
                    "{}::write_source({value}, w{args})?;",
                    child.type_name
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
    kind: FlagValue,
}

enum FlagValue {
    Number,
    Bool,
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
                let bits = match derived.kind {
                    FlagValue::Bool => format!("{ty}::from({})", derived.value),
                    FlagValue::Number => {
                        // A number the flag's bit-range cannot hold is not a frame.
                        lines.push(format!(
                            "if {} > 0x{width_mask:X} {{\n    return Err(CodecError::LengthOutOfRange);\n}}",
                            derived.value
                        ));
                        let narrowed = format!(
                            "_v_{}_{}",
                            plan.l().codec_field_id(&carrier.id),
                            filters::to_snake_case(derived.flag.clone())
                        );
                        lines.push(format!(
                            "let {narrowed}: {ty} = {ty}::try_from({}).map_err(|_| CodecError::LengthOutOfRange)?;",
                            derived.value
                        ));
                        narrowed
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
            let binding = if sets.is_empty() { "let" } else { "let mut" };
            lines.push(format!("{binding} {local}: {ty} = {value};"));
            for def in sets {
                lines.push(format!(
                    "if let Some(_o) = over.{member} {{\n    {local} = ({local} & !0x{mask:X}) | ({ty}::from(_o) << {bit});\n}}",
                    member = plan.override_member(carrier, &def.name),
                    mask = 1u64 << def.bit,
                    bit = def.bit,
                ));
            }
        }
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
