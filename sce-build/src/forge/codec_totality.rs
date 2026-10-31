// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Whether a codec's encode can refuse a value, and why (docs/adr/0065 §2).
//!
//! An encode is a printer. Over its abstract syntax (the source a value
//! presents, docs/adr/0058 §1) it is total exactly when every value the
//! abstract syntax admits has a frame. Where the abstract syntax is wider than
//! the encodable set, the encode refuses the difference: a length past the
//! field that carries it, a flag that disagrees with the presence it announces.
//! This module names each of
//! those places, element by element, from the codec model and its roles alone,
//! so it is the one statement of when an encode can refuse. A codec's
//! declaration (`sce:encode-may-fail`) is held to it before any backend renders
//! (docs/adr/0065 §3), and the Rust encode writes a refusal exactly where it
//! names one ([`Totality::require`]).
//!
//! A conversion that cannot fail is not a refusal: a length or a count widens
//! into a 64-bit field on every target (docs/adr/0065 §5), a count bounded by
//! its list's `max-count` fits a field that holds the bound, and a tag fits the
//! bits it is written into: an arm's is declared there, and a default arm holds
//! an [`UnknownTag`] (docs/adr/0065 §6 R3). A list is no refusal either: its
//! source presents it as a `BoundedIter`, which holds no more entries than the
//! list's bound ([`list_bound`], docs/adr/0065 §6 R2).

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt;

use crate::forge::codec_roles::{self, CodecRoles, Derivation, ElementId, InputImports, Role};
use crate::forge::error::{ForgeError, ValidationError};
use crate::forge::generator::ImportContext;
use crate::forge::model::{
    BitSize, CborEntry, CodecEncoding, CodecField, CodecModel, ContentLineEntry, ContentLineModel,
    FlagBind, SceType,
};

/// What an encode refuses a value with: the `CodecError` it returns, or a codec
/// it holds refusing first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RefusalKind {
    /// A length or a count past the field or bits that carry it, or a
    /// length the wire holds less an offset (`sce:length-arith`) that would be
    /// negative.
    LengthOutOfRange,
    /// A gated field whose presence disagrees with a gate the encoder cannot
    /// derive.
    PresentIfMismatch,
    /// Two things a value says of one number disagree.
    InconsistentSource,
    /// A CBOR text or byte string past its `sce:max-size`, or an enum value no
    /// unsigned integer carries.
    CborOutOfRange,
    /// A CBOR byte string of another length than its `sce:length`.
    CborWrongLength,
    /// A content-line value its line cannot carry: past its size, a control
    /// character, a variant with no text.
    LineBadValue,
    /// A content-line list or property a line requires, absent.
    LineRequiredMissing,
    /// A codec this one holds refuses.
    Child,
}

impl RefusalKind {
    /// The `CodecError` variant the encode returns, or `None` for a refusal
    /// a child returns itself.
    pub fn error_variant(self) -> Option<&'static str> {
        Some(match self {
            RefusalKind::LengthOutOfRange => "LengthOutOfRange",
            RefusalKind::PresentIfMismatch => "PresentIfMismatch",
            RefusalKind::InconsistentSource => "InconsistentSource",
            RefusalKind::CborOutOfRange => "CborOutOfRange",
            RefusalKind::CborWrongLength => "CborWrongLength",
            RefusalKind::LineBadValue => "LineBadValue",
            RefusalKind::LineRequiredMissing => "LineRequiredMissing",
            RefusalKind::Child => return None,
        })
    }
}

impl fmt::Display for RefusalKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            self.error_variant()
                .unwrap_or("a refusal of a codec it holds"),
        )
    }
}

/// One place an encode can refuse a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub kind: RefusalKind,
    /// The element, as the codec names it: a field id, `carrier.flag`, an
    /// entry id, `<variant>`, or the import alias of a codec it holds.
    pub element: String,
    /// What the value can say that the encoding cannot carry.
    pub reason: String,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}`: {} ({})", self.element, self.reason, self.kind)
    }
}

/// Every place the encode of one codec can refuse a value, in the order the
/// encode meets them.
#[derive(Debug, Default)]
pub struct Totality {
    refusals: Vec<Refusal>,
    /// The refusals an emitter has written, by kind and element: an emitter
    /// that writes one this analysis does not name, or names one it never
    /// writes, disagrees with the one statement of when the encode refuses.
    written: RefCell<BTreeSet<(RefusalKind, String)>>,
}

impl Totality {
    /// Whether the encode refuses no value.
    pub fn is_total(&self) -> bool {
        self.refusals.is_empty()
    }

    /// The places the encode can refuse a value.
    pub fn refusals(&self) -> &[Refusal] {
        &self.refusals
    }

    /// Whether the encode refuses at `element` with `kind`.
    pub fn refuses(&self, kind: RefusalKind, element: &str) -> bool {
        self.refusals
            .iter()
            .any(|r| r.kind == kind && r.element == element)
    }

    /// The `CodecError::…` an emitter writes where the encode refuses at
    /// `element` with `kind`, recorded as written.
    ///
    /// Panics when this analysis names no such refusal: the emitter is about
    /// to write a refusal of a codec the analysis calls total, or one the
    /// declaration was not held to. That is a defect of the generator, not of
    /// the document.
    pub fn require(&self, kind: RefusalKind, element: &str) -> String {
        assert!(
            self.refuses(kind, element),
            "the encode writes a {kind} refusal at `{element}` that the totality analysis \
             does not name (docs/adr/0065 §2)"
        );
        self.written
            .borrow_mut()
            .insert((kind, element.to_string()));
        match kind.error_variant() {
            Some(variant) => format!("CodecError::{variant}"),
            None => String::new(),
        }
    }

    /// Panics when a refusal this analysis names was never written by the
    /// emitter that rendered the encode ([`Self::require`]). Called once the
    /// encode is rendered.
    pub fn assert_all_written(&self, codec: &str) {
        let written = self.written.borrow();
        for r in &self.refusals {
            assert!(
                written.contains(&(r.kind, r.element.clone())),
                "codec `{codec}`: the totality analysis names {r}, which the encode never \
                 writes (docs/adr/0065 §2)"
            );
        }
    }
}

/// What the analysis asks of the codecs and enums a codec imports.
pub trait Imports: InputImports {
    /// Whether the encode of the codec imported as `alias` can refuse.
    fn codec_may_fail(&mut self, alias: &str) -> bool;
    /// The carrier type of the enum imported as `alias`.
    fn enum_carrier(&mut self, alias: &str) -> Option<SceType>;
}

/// The imports a codec's generation resolved: each codec import says whether
/// its encode can refuse, and each enum import its carrier.
pub struct Resolved<'a>(pub &'a [ImportContext]);

impl InputImports for Resolved<'_> {
    fn flag_binds(&mut self, alias: &str) -> Vec<FlagBind> {
        codec_roles::Contexts(self.0).flag_binds(alias)
    }

    fn says_input(&mut self, alias: &str, input: &str) -> bool {
        codec_roles::Contexts(self.0).says_input(alias, input)
    }
}

impl Imports for Resolved<'_> {
    fn codec_may_fail(&mut self, alias: &str) -> bool {
        self.0
            .iter()
            .any(|i| i.alias == alias && i.kind == "codec" && i.codec_source_can_refuse)
    }

    fn enum_carrier(&mut self, alias: &str) -> Option<SceType> {
        crate::forge::cbor_codec::enum_carrier(self.0, alias)
    }
}

/// Whether the encode of `m`, whose imports are resolved as `imports`, can
/// refuse a value.
pub fn may_fail(m: &CodecModel, imports: &[ImportContext]) -> bool {
    !analyse(m, &mut Resolved(imports)).is_total()
}

/// Hold `m`'s statement of its encode's totality (`sce:encode-may-fail`) to
/// the refusals the encode has (docs/adr/0065 §3): a codec that states it is
/// total and refuses somewhere is refused at the first place it refuses, and
/// one that states it may fail and refuses nowhere is refused at its root.
pub fn check(m: &CodecModel, imports: &[ImportContext]) -> Result<(), ForgeError> {
    let totality = analyse(m, &mut Resolved(imports));
    match (m.encode_may_fail, totality.refusals().first()) {
        (false, Some(r)) => Err(
            ForgeError::from(ValidationError::CodecUndeclaredEncodeRefusal {
                codec: m.name.clone(),
                element: r.element.clone(),
                kind: r.kind.to_string(),
                reason: r.reason.clone(),
            })
            .at_line(element_line(m, &r.element)),
        ),
        (true, None) => Err(
            ForgeError::from(ValidationError::CodecEncodeMayFailWithoutRefusal {
                codec: m.name.clone(),
            })
            .at_line(m.source_location.as_ref().and_then(|l| l.line)),
        ),
        _ => Ok(()),
    }
}

/// The row an element of `m` is declared at: a field's (or a flag's carrier's),
/// a CBOR or content-line entry's, the root's for anything else.
fn element_line(m: &CodecModel, element: &str) -> Option<u32> {
    let field = element.split('.').next().unwrap_or(element);
    m.fields
        .iter()
        .find(|f| f.id == field)
        .and_then(|f| f.line)
        .or_else(|| {
            m.content_line
                .iter()
                .flat_map(|c| c.entries.iter())
                .find(|e| e.id == element)
                .and_then(|e| e.line)
        })
        .or_else(|| m.source_location.as_ref().and_then(|l| l.line))
}

/// The places the encode of `m` can refuse a value.
pub fn analyse(m: &CodecModel, imports: &mut dyn Imports) -> Totality {
    let mut refusals = Vec::new();
    match m.encoding {
        CodecEncoding::Positional => positional(m, imports, &mut refusals),
        CodecEncoding::Cbor => cbor(&m.cbor_entries, imports, &mut refusals),
        CodecEncoding::ContentLine => {
            if let Some(model) = &m.content_line {
                content_line(model, &mut refusals);
            }
        }
    }
    Totality {
        refusals,
        written: RefCell::default(),
    }
}

// ── Positional ────────────────────────────────────────────────────────

/// The largest value a derived number can take, or `None` when what it
/// measures is unbounded in the abstract syntax.
fn number_bound(m: &CodecModel, by: &Derivation) -> Option<u64> {
    match by {
        // A list is counted against its bound before its count is derived, so
        // the count never exceeds the largest of the bounds it is shared by.
        Derivation::CountOf { lists } => lists
            .iter()
            .map(|id| {
                m.fields
                    .iter()
                    .find(|f| &f.id == id)
                    .map(list_bound)
                    .unwrap_or(u64::MAX)
            })
            .max(),
        // An input is one bit (docs/adr/0065 §6 R4a).
        Derivation::BoundInput { .. } => Some(1),
        _ => None,
    }
}

/// The number of entries the list `field` is held to.
pub fn list_bound(field: &CodecField) -> u64 {
    match &field.bit_size {
        BitSize::TlvChain { max_depth, .. } => u64::from(*max_depth),
        _ => u64::from(crate::forge::limits::resolve_max_count(field.max_count)),
    }
}

/// The width in bits of an unsigned integer type, or `None` for another type.
fn unsigned_bits(ty: &SceType) -> Option<u32> {
    Some(match ty {
        SceType::Uint8 => 8,
        SceType::Uint16 => 16,
        SceType::Uint32 => 32,
        SceType::Uint64 => 64,
        _ => return None,
    })
}

/// The largest value an integer type holds, or `None` for another type.
fn largest_of(ty: &SceType) -> Option<u64> {
    Some(match ty {
        SceType::Uint8 => u64::from(u8::MAX),
        SceType::Uint16 => u64::from(u16::MAX),
        SceType::Uint32 => u64::from(u32::MAX),
        SceType::Uint64 => u64::MAX,
        SceType::Int8 => i8::MAX as u64,
        SceType::Int16 => i16::MAX as u64,
        SceType::Int32 => i32::MAX as u64,
        SceType::Int64 => i64::MAX as u64,
        _ => return None,
    })
}

/// Whether a derived number (a `usize`) at most `bound` always fits a field
/// whose type holds at most `largest`: a 64-bit unsigned field holds every
/// `usize` on every target Rust supports (docs/adr/0065 §5), and another holds
/// a number whose bound is no larger than its own.
pub fn number_fits(largest: Option<u64>, bound: Option<u64>) -> bool {
    match (largest, bound) {
        (Some(u64::MAX), _) => true,
        (Some(largest), Some(bound)) => bound <= largest,
        _ => false,
    }
}

/// The largest value of `bits` bits.
fn mask_of(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
}

/// The bit-range a variant's tag is read from, when it is one: a flag of the
/// peeked byte or of the carrier field.
fn tag_flag(m: &CodecModel) -> Option<&crate::forge::model::FlagDef> {
    let variant = m.variant.as_ref()?;
    match (&variant.peek_byte, &variant.tag_flag) {
        (Some(peek), Some(flag)) => peek.flags.iter().find(|f| &f.name == flag),
        (None, Some(flag)) => variant
            .tag_field
            .as_ref()
            .and_then(|id| m.fields.iter().find(|f| &f.id == id))
            .and_then(|carrier| carrier.flags.iter().find(|f| &f.name == flag)),
        _ => None,
    }
}

/// The type a variant's tag is held in: the width of the bit-range it is read
/// from, or the carrier field's own type.
pub fn tag_type(m: &CodecModel) -> SceType {
    let Some(variant) = &m.variant else {
        return SceType::Uint8;
    };
    match tag_flag(m) {
        Some(def) => match def.width.max(1) {
            0..=8 => SceType::Uint8,
            9..=16 => SceType::Uint16,
            17..=32 => SceType::Uint32,
            _ => SceType::Uint64,
        },
        None => variant
            .tag_field
            .as_ref()
            .and_then(|id| m.fields.iter().find(|f| &f.id == id))
            .map_or(SceType::Uint8, |carrier| carrier.sce_type.clone()),
    }
}

/// The tags a variant's default arm may hold (docs/adr/0065 §6 R3): each value
/// inside the bits the tag is read from and outside every arm's tag.
///
/// The arm holds its tag as a type that admits these values and no other, so a
/// default arm is written as it is and read back as the default arm. Its encode
/// has nothing to refuse, and the analysis names no refusal for the tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownTag {
    /// The type the tag is held in ([`tag_type`]).
    pub ty: SceType,
    /// The bits the tag is read from: the bit-range's width, or the type's.
    pub bits: u32,
    /// The tags the arms name, which a default arm may not hold, in the order
    /// the document declares them. An arm under a build feature is among them in
    /// every build: a build without the feature refuses its tag on decode, and
    /// the default arm never holds it.
    pub known: Vec<u64>,
}

impl UnknownTag {
    /// The largest tag the bits hold.
    pub fn max(&self) -> u64 {
        mask_of(self.bits)
    }

    /// The largest tag the bits hold, when it is smaller than the largest value
    /// of the type, so that a tag of the type can be past it.
    pub fn max_below_type(&self) -> Option<u64> {
        let type_bits = unsigned_bits(&self.ty).unwrap_or(64);
        (self.bits < type_bits).then(|| self.max())
    }

    /// Whether `tag` is one a default arm may hold.
    pub fn admits(&self, tag: u64) -> bool {
        tag <= self.max() && !self.known.contains(&tag)
    }

    /// The known tags as runs of consecutive values, each `(first, last)`, in
    /// ascending order: a backend writes a run as one range.
    pub fn known_runs(&self) -> Vec<(u64, u64)> {
        let mut tags = self.known.clone();
        tags.sort_unstable();
        tags.dedup();
        let mut runs: Vec<(u64, u64)> = Vec::new();
        for tag in tags {
            match runs.last_mut() {
                Some((_, last)) if last.checked_add(1) == Some(tag) => *last = tag,
                _ => runs.push((tag, tag)),
            }
        }
        runs
    }
}

/// The tags `m`'s default arm may hold, or `None` when its variant has no
/// default arm.
pub fn unknown_tag(m: &CodecModel) -> Option<UnknownTag> {
    let variant = m.variant.as_ref()?;
    variant.default_arm.as_ref()?;
    let ty = tag_type(m);
    let bits = match tag_flag(m) {
        Some(def) => def.width.max(1),
        None => unsigned_bits(&ty).unwrap_or(64),
    };
    Some(UnknownTag {
        ty,
        bits,
        known: variant.arms.iter().map(|a| a.value).collect(),
    })
}

/// The fields whose size the element `id` is derived from, when it is a
/// length or a count.
pub fn measured<'m>(m: &'m CodecModel, roles: &CodecRoles, id: &ElementId) -> Vec<&'m CodecField> {
    let Some(Role::Derived(by)) = roles.role_of(id) else {
        return Vec::new();
    };
    let names: Vec<&str> = by
        .iter()
        .flat_map(|d| match d {
            Derivation::LengthOf { contents } => contents
                .iter()
                .map(|c| c.field.as_str())
                .collect::<Vec<_>>(),
            Derivation::CountOf { lists } => lists.iter().map(String::as_str).collect(),
            _ => Vec::new(),
        })
        .collect();
    m.fields
        .iter()
        .filter(|f| names.contains(&f.id.as_str()))
        .collect()
}

/// Whether a source presents `field`: it is not derived whole.
pub fn presented(roles: &CodecRoles, field: &CodecField) -> bool {
    !matches!(
        roles.role_of(&ElementId::Field(field.id.clone())),
        Some(Role::Derived(_))
    )
}

/// Whether the element `id` is derived only while the fields it measures are
/// there: a length or a count of content that is gated has nothing to be
/// derived from when the gate is off, and the wire still holds a number, which
/// the source states (docs/adr/0058 §1, "a length or count of gated content").
pub fn stated_when_absent(m: &CodecModel, roles: &CodecRoles, id: &ElementId) -> bool {
    let measured = measured(m, roles, id);
    !measured.is_empty() && measured.iter().all(|f| f.present_if.is_some())
}

/// Whether a source states `field` while its content is absent
/// ([`stated_when_absent`]): a length or a count, not gated itself.
pub fn stated(m: &CodecModel, roles: &CodecRoles, field: &CodecField) -> bool {
    !presented(roles, field)
        && field.present_if.is_none()
        && stated_when_absent(m, roles, &ElementId::Field(field.id.clone()))
}

/// The import alias of the codec a list or an embedded body holds.
pub fn child_alias(field: &CodecField) -> Option<&str> {
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

fn positional(m: &CodecModel, imports: &mut dyn Imports, out: &mut Vec<Refusal>) {
    // A parent-tag derivation fills its bits with the arm's value masked to
    // them, so it never refuses; the analysis does not need the imports it is
    // read from. The inputs the held codecs say it does need.
    let roles = codec_roles::classify_with(m, &[], &mut *imports);
    let push = |out: &mut Vec<Refusal>, kind, element: &str, reason: String| {
        out.push(Refusal {
            kind,
            element: element.to_string(),
            reason,
        });
    };

    // A list is no refusal: its source holds no more entries than its bound
    // (docs/adr/0065 §6 R2, `list_bound`).
    for (id, by) in roles.derived() {
        for derivation in by {
            derived(m, &roles, id, derivation, out);
        }
    }

    // The gates the encoder cannot derive a flag from. A gate on an input the
    // value says is derived (docs/adr/0065 §6 R4a).
    for check in roles.source_checks() {
        if m.fields
            .iter()
            .any(|f| f.id == check.field && f.present_if.is_some())
        {
            push(
                out,
                RefusalKind::PresentIfMismatch,
                &check.field,
                format!(
                    "the field may be given where `{}` says it is absent, or left out where it \
                     says it is there",
                    check.gate
                ),
            );
        }
    }

    // A held codec says an input that this one hands it a value of from
    // elsewhere: the two may disagree.
    for check in &roles.input_checks {
        push(
            out,
            RefusalKind::PresentIfMismatch,
            check.binder.site.element(&check.binder.alias),
            format!(
                "`{}` says its input `{}` itself, and may say it otherwise than {}",
                check.binder.alias, check.binder.input, check.against
            ),
        );
    }

    // A codec it holds refuses, and then so does this one.
    for field in &m.fields {
        if let Some(alias) = child_alias(field) {
            if imports.codec_may_fail(alias) {
                push(
                    out,
                    RefusalKind::Child,
                    &field.id,
                    format!("it holds `{alias}`, whose encode can refuse"),
                );
            }
        }
    }
    if let Some(variant) = &m.variant {
        for arm in variant.arms.iter().chain(variant.default_arm.iter()) {
            if imports.codec_may_fail(&arm.body_alias) {
                push(
                    out,
                    RefusalKind::Child,
                    &arm.body_alias,
                    format!("an arm holds `{}`, whose encode can refuse", arm.body_alias),
                );
            }
        }
    }
}

/// The refusals of one derivation of the element `id`.
fn derived(
    m: &CodecModel,
    roles: &CodecRoles,
    id: &ElementId,
    by: &Derivation,
    out: &mut Vec<Refusal>,
) {
    let element = id.to_string();
    let push = |out: &mut Vec<Refusal>, kind, element: &str, reason: String| {
        out.push(Refusal {
            kind,
            element: element.to_string(),
            reason,
        });
    };
    let field_of = |name: &str| m.fields.iter().find(|f| f.id == name);
    // What the number is measured from, each always there or gated.
    let gated = |name: &str| field_of(name).is_some_and(|f| f.present_if.is_some());
    let sources: Vec<(String, bool)> = match by {
        Derivation::LengthOf { contents } => {
            for content in contents.iter().filter(|c| c.arith > 0) {
                push(
                    out,
                    RefusalKind::LengthOutOfRange,
                    &content.field,
                    format!(
                        "the wire holds its length less {}, so it may not be shorter than that",
                        content.arith
                    ),
                );
            }
            contents
                .iter()
                .filter(|c| field_of(&c.field).is_some())
                .map(|c| (c.field.clone(), gated(&c.field)))
                .collect()
        }
        Derivation::CountOf { lists } => lists
            .iter()
            .filter(|l| field_of(l).is_some())
            .map(|l| (l.clone(), gated(l)))
            .collect(),
        // The length of a body that is not there is the wire's zero, so a
        // gated body measures as one that is always there.
        Derivation::EncodedLengthOf { bodies } => bodies
            .iter()
            .filter(|b| field_of(b).is_some())
            .map(|b| (b.clone(), false))
            .collect(),
        Derivation::PresenceOf { gated: fields } => {
            if fields.len() > 1 {
                push(
                    out,
                    RefusalKind::InconsistentSource,
                    &element,
                    format!(
                        "the fields it announces ({}) may disagree about whether it is set",
                        fields
                            .iter()
                            .map(|g| format!("`{}`", g.field))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                );
            }
            return;
        }
        // The input each body or arm says. The arms of one variant are one
        // source, which is there when one of them is.
        Derivation::BoundInput { binders } => {
            let mut sources: Vec<(String, bool)> = Vec::new();
            for binder in binders {
                match &binder.site {
                    codec_roles::BindSite::Embed { field } => {
                        sources.push((field.clone(), gated(field)));
                    }
                    codec_roles::BindSite::Arm => {
                        if !sources.iter().any(|(s, _)| s == "<variant>") {
                            sources.push(("<variant>".to_string(), true));
                        }
                    }
                    codec_roles::BindSite::List { .. } => {}
                }
            }
            sources
        }
        // A known arm's tag is declared inside the tag's bits, and a default
        // arm's is an [`UnknownTag`], which holds no other (docs/adr/0065 §6 R3).
        Derivation::VariantTag { .. }
        | Derivation::ParentTag { .. }
        | Derivation::ChainContinuation { .. } => return,
    };

    // Several sources of one number agree, or no frame says all of them.
    let always = sources.iter().filter(|(_, g)| !g).count();
    let gated_count = sources.len() - always;
    if always >= 2 || (always >= 1 && gated_count >= 1) || (always == 0 && gated_count >= 2) {
        push(
            out,
            RefusalKind::InconsistentSource,
            &element,
            format!(
                "it is derived from {}, which may disagree in size",
                sources
                    .iter()
                    .map(|(s, _)| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }

    let bound = number_bound(m, by);
    let free = always == 0 && gated_count >= 1;
    match id {
        ElementId::Field(name) => {
            let Some(field) = field_of(name) else {
                return;
            };
            // A 64-bit unsigned field holds every length and count; any other
            // holds the number only where the number's bound is no larger.
            let largest = largest_of(&field.sce_type);
            if !number_fits(largest, bound) {
                push(
                    out,
                    RefusalKind::LengthOutOfRange,
                    &element,
                    match largest {
                        Some(n) => {
                            format!("the number may be past {n}, the largest its field holds")
                        }
                        None => "the number may be one its field's type does not hold".to_string(),
                    },
                );
            }
            if free && stated(m, roles, field) {
                push(
                    out,
                    RefusalKind::InconsistentSource,
                    &element,
                    "the value may state a number beside content that says another".to_string(),
                );
            }
        }
        ElementId::Flag { .. } => {
            let width = flag_width(m, id);
            if width < 64 && bound.is_none_or(|b| b > mask_of(width)) {
                push(
                    out,
                    RefusalKind::LengthOutOfRange,
                    &element,
                    format!(
                        "the number may be past {}, the largest its {width} bits hold",
                        mask_of(width)
                    ),
                );
            }
        }
        ElementId::Variant => {}
    }
}

/// The width of the bit-range `id` names.
fn flag_width(m: &CodecModel, id: &ElementId) -> u32 {
    let ElementId::Flag { field, flag } = id else {
        return 64;
    };
    m.fields
        .iter()
        .find(|f| &f.id == field)
        .and_then(|carrier| carrier.flags.iter().find(|d| &d.name == flag))
        .map_or(64, |def| def.width)
}

// ── CBOR ──────────────────────────────────────────────────────────────

fn cbor(entries: &[CborEntry], imports: &mut dyn Imports, out: &mut Vec<Refusal>) {
    for e in entries {
        let refusal = match crate::forge::cbor_codec::kind(e) {
            "text" | "bytes" => e.max_size.map(|max| {
                (
                    RefusalKind::CborOutOfRange,
                    format!("the value may be longer than {max}, its max-size"),
                )
            }),
            "bytes_exact" => Some((
                RefusalKind::CborWrongLength,
                format!(
                    "the value may be another length than {}, its length",
                    e.length.unwrap_or_default()
                ),
            )),
            "enum" => match e.present_type() {
                SceType::Enum(r) => match imports.enum_carrier(&r.alias) {
                    Some(SceType::Int8 | SceType::Int16 | SceType::Int32 | SceType::Int64) => {
                        Some((
                            RefusalKind::CborOutOfRange,
                            "the enum's carrier is signed, and a negative value has no \
                             unsigned integer"
                                .to_string(),
                        ))
                    }
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };
        if let Some((kind, reason)) = refusal {
            out.push(Refusal {
                kind,
                element: e.id.clone(),
                reason,
            });
        }
    }
}

// ── Content line ──────────────────────────────────────────────────────

fn content_line(model: &ContentLineModel, out: &mut Vec<Refusal>) {
    let mut push = |kind, e: &ContentLineEntry, reason: &str| {
        out.push(Refusal {
            kind,
            element: e.id.clone(),
            reason: reason.to_string(),
        });
    };
    for e in &model.entries {
        let is_param = e.param.is_some();
        if !is_param {
            let required_list =
                (model.reads_records(e) || e.max_count.is_some()) && e.is_required();
            let optional_with_params = !model.reads_records(e)
                && e.max_count.is_none()
                && !e.is_required()
                && model.parameters_of(e).next().is_some();
            if required_list || optional_with_params {
                push(
                    RefusalKind::LineRequiredMissing,
                    e,
                    if required_list {
                        "the list may be empty where its line is required"
                    } else {
                        "a parameter may be given without the property it stands on"
                    },
                );
            }
        }
        if writes_refusing_value(e) {
            push(
                RefusalKind::LineBadValue,
                e,
                "the value may hold what its line cannot carry: past its size, a control \
                 character, a variant with no text",
            );
        }
    }
}

/// Whether the line writer refuses some value of `e`: a text, a list of
/// values, or a variant, whose text the writer looks up.
fn writes_refusing_value(e: &ContentLineEntry) -> bool {
    e.separator.is_some()
        || matches!(e.present_type(), SceType::String | SceType::Enum(_))
        || e.param.is_some() && !matches!(e.present_type(), SceType::Enum(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoImports;

    impl InputImports for NoImports {
        fn flag_binds(&mut self, _alias: &str) -> Vec<FlagBind> {
            Vec::new()
        }
        fn says_input(&mut self, _alias: &str, _input: &str) -> bool {
            false
        }
    }

    impl Imports for NoImports {
        fn codec_may_fail(&mut self, _alias: &str) -> bool {
            false
        }
        fn enum_carrier(&mut self, _alias: &str) -> Option<SceType> {
            None
        }
    }

    fn model(xml: &str) -> CodecModel {
        let label = crate::DocumentLabel {
            identifier: "t",
            diagnostic_label: ".scxml-fixture",
        };
        match crate::forge::parser::parse_forge(xml, label) {
            Ok(Some(crate::forge::model::ForgeDocument::Codec(m))) => m,
            Ok(_) => panic!("the fixture is not a codec document"),
            Err(located) => panic!("the fixture does not parse: {:?}", located.error),
        }
    }

    fn kinds(m: &CodecModel) -> Vec<(RefusalKind, String)> {
        analyse(m, &mut NoImports)
            .refusals()
            .iter()
            .map(|r| (r.kind, r.element.clone()))
            .collect()
    }

    #[test]
    fn a_codec_of_scalars_is_total() {
        let m = model(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                       sce:kind="codec" name="t"><datamodel>
                 <sce:field id="a" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
                 <sce:field id="b" sce:type="uint32" sce:byte="1" sce:bit-size="32"/>
               </datamodel></scxml>"#,
        );
        assert!(analyse(&m, &mut NoImports).is_total());
    }

    #[test]
    fn a_length_narrower_than_its_content_refuses_and_a_64_bit_one_does_not() {
        let narrow = model(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                       sce:kind="codec" name="t"><datamodel>
                 <sce:field id="len" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
                 <sce:field id="data" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
                            sce:length-field="len" sce:max-size="255"/>
               </datamodel></scxml>"#,
        );
        assert_eq!(
            kinds(&narrow),
            vec![(RefusalKind::LengthOutOfRange, "len".to_string())]
        );
        let wide = model(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                       sce:kind="codec" name="t"><datamodel>
                 <sce:field id="len" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
                 <sce:field id="data" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
                            sce:length-field="len" sce:max-size="255"/>
               </datamodel></scxml>"#,
        );
        assert!(analyse(&wide, &mut NoImports).is_total());
    }

    #[test]
    fn a_disjunction_gate_is_a_check_the_encode_refuses_at() {
        let m = model(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                       sce:kind="codec" name="t"><datamodel>
                 <sce:flags id="flags" sce:type="uint8" sce:byte="0" sce:bit-size="8">
                   <sce:flag name="a" bit="0"/><sce:flag name="b" bit="1"/>
                 </sce:flags>
                 <sce:field id="seq" sce:type="uint16" sce:byte="1" sce:bit-size="16"
                            sce:present-if="flags.a || flags.b"/>
               </datamodel></scxml>"#,
        );
        assert_eq!(
            kinds(&m),
            vec![(RefusalKind::PresentIfMismatch, "seq".to_string())]
        );
    }

    #[test]
    fn a_one_flag_gate_is_derived_and_does_not_refuse() {
        let m = model(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                       sce:kind="codec" name="t"><datamodel>
                 <sce:flags id="flags" sce:type="uint8" sce:byte="0" sce:bit-size="8">
                   <sce:flag name="a" bit="0"/>
                 </sce:flags>
                 <sce:field id="seq" sce:type="uint16" sce:byte="1" sce:bit-size="16"
                            sce:present-if="flags.a"/>
               </datamodel></scxml>"#,
        );
        assert!(analyse(&m, &mut NoImports).is_total());
    }

    /// A default arm holds a tag inside the bits the tag is read from and
    /// outside every arm's tag, so the tag it writes is not a refusal
    /// (docs/adr/0065 §6 R3).
    #[test]
    fn a_default_arm_holds_only_a_tag_its_bits_hold_and_no_arm_names() {
        let m = model(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                       sce:kind="codec" name="t">
                 <sce:import src="a.scxml" kind="codec" as="a"/>
                 <sce:import src="b.scxml" kind="codec" as="b"/>
                 <datamodel>
                 <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
                   <sce:flag name="id" bit="0" width="4"/>
                   <sce:flag name="enc" bit="5" width="2"/>
                 </sce:flags>
                 <sce:variant tag="header.enc">
                   <sce:arm value="0x00" type="a" default="true"/>
                   <sce:arm value="0x01" type="b"/>
                   <sce:arm value="0x02" type="b"/>
                   <sce:default type="a"/>
                 </sce:variant>
               </datamodel></scxml>"#,
        );
        let unknown = unknown_tag(&m).expect("the variant has a default arm");
        assert_eq!(
            unknown,
            UnknownTag {
                ty: SceType::Uint8,
                bits: 2,
                known: vec![0, 1, 2],
            }
        );
        assert_eq!(unknown.known_runs(), vec![(0, 2)]);
        assert_eq!(unknown.max_below_type(), Some(3));
        assert_eq!(
            (0..=4).filter(|&t| unknown.admits(t)).collect::<Vec<_>>(),
            vec![3]
        );
        assert!(analyse(&m, &mut NoImports).is_total());
    }

    /// Runs of consecutive tags are written as one range, and the gaps between
    /// them break the runs.
    #[test]
    fn the_known_tags_are_grouped_into_runs_of_consecutive_values() {
        let unknown = UnknownTag {
            ty: SceType::Uint8,
            bits: 8,
            known: vec![7, 1, 2, 3, 9, 8, 255],
        };
        assert_eq!(unknown.known_runs(), vec![(1, 3), (7, 9), (255, 255)]);
        assert_eq!(unknown.max_below_type(), None);
    }

    #[test]
    fn a_codec_that_holds_a_refusing_codec_refuses() {
        struct Refusing;
        impl InputImports for Refusing {
            fn flag_binds(&mut self, _alias: &str) -> Vec<FlagBind> {
                Vec::new()
            }
            fn says_input(&mut self, _alias: &str, _input: &str) -> bool {
                false
            }
        }
        impl Imports for Refusing {
            fn codec_may_fail(&mut self, alias: &str) -> bool {
                alias == "inner"
            }
            fn enum_carrier(&mut self, _alias: &str) -> Option<SceType> {
                None
            }
        }
        let m = model(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                       sce:kind="codec" name="t">
                 <sce:import kind="codec" src="inner.scxml" as="inner"/>
                 <datamodel>
                 <sce:field id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
                 <sce:embed id="body" type="inner" sce:byte="1"/>
               </datamodel></scxml>"#,
        );
        let t = analyse(&m, &mut Refusing);
        assert_eq!(
            t.refusals()
                .iter()
                .map(|r| (r.kind, r.element.as_str()))
                .collect::<Vec<_>>(),
            vec![(RefusalKind::Child, "body")]
        );
    }

    /// A one-byte length over bytes: an encode that refuses, stated as `root`.
    fn length_codec(root: &str) -> CodecModel {
        model(&format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                       sce:kind="codec" {root} name="t"><datamodel>
                 <sce:field id="len" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
                 <sce:field id="data" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
                            sce:length-field="len" sce:max-size="255"/>
               </datamodel></scxml>"#
        ))
    }

    /// Scalars alone: an encode that refuses nothing, stated as `root`.
    fn scalar_codec(root: &str) -> CodecModel {
        model(&format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
                       sce:kind="codec" {root} name="t"><datamodel>
                 <sce:field id="a" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
               </datamodel></scxml>"#
        ))
    }

    fn validation(err: ForgeError) -> ValidationError {
        match err.into_positioned().0 {
            ForgeError::Validation(v) => *v,
            other => panic!("not a validation error: {other:?}"),
        }
    }

    #[test]
    fn a_codec_that_refuses_and_states_it_is_total_is_refused_where_it_refuses() {
        let err = check(&length_codec(""), &[]).expect_err("the statement is untrue");
        match validation(err) {
            ValidationError::CodecUndeclaredEncodeRefusal { element, kind, .. } => {
                assert_eq!(
                    (element.as_str(), kind.as_str()),
                    ("len", "LengthOutOfRange")
                );
            }
            other => panic!("another refusal: {other:?}"),
        }
        check(&length_codec(r#"sce:encode-may-fail="true""#), &[])
            .expect("a true statement is accepted");
    }

    #[test]
    fn a_codec_that_refuses_nothing_and_states_it_may_fail_is_refused() {
        let err = check(&scalar_codec(r#"sce:encode-may-fail="true""#), &[])
            .expect_err("the statement is untrue");
        assert!(matches!(
            validation(err),
            ValidationError::CodecEncodeMayFailWithoutRefusal { .. }
        ));
        check(&scalar_codec(""), &[]).expect("a true statement is accepted");
    }
}
