// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Single source of truth for `#[derive(...)]` attributes emitted by
//! forge Rust codegen templates.
//!
//! Before this module, each `tools/codegen/templates/forge/rust/*.rs.jinja2`
//! template hardcoded its own derive list inline. Cross-cutting policy
//! changes (adding `Debug` everywhere, adding `#[non_exhaustive]`,
//! gating on a new cargo feature) had to be applied in N places, with
//! no compile-time check that N stayed in sync. Drift was silent: a
//! missed template still rendered and compiled.
//!
//! The categories here partition every wire-typed struct or enum the
//! Rust forge templates emit. Each render function in
//! [`crate::forge::generator`] injects [`derives_attr`] for its
//! category into the jinja context under a category-named key (e.g.
//! `codec_struct_derives_attr`), and the template renders the result
//! verbatim — templates carry no policy of their own.
//!
//! Adding a new emitted struct/enum:
//!   1. Add a [`RustDeriveCategory`] variant with the textbook derive
//!      list for that wire shape (mirror existing wire-typed payload
//!      policy when the shape is plain data).
//!   2. Inject `<key>_derives_attr` into the matching render function's
//!      jinja context (Rust language arm only).
//!   3. Replace the template's hardcoded `#[derive(...)]` line with
//!      `{{ <key>_derives_attr }}`.
//!   4. Extend `sce-build/tests/forge_codegen_derive_ssot.rs` to lock
//!      the new category.

/// Category of forge-emitted Rust struct or enum, partitioned by wire
/// role rather than by template file (one template may emit multiple
/// categories, e.g. `codec.rs.jinja2` emits both [`Self::CodecStruct`]
/// and [`Self::CodecVariantEnum`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustDeriveCategory {
    /// `codec.rs.jinja2` — `pub struct {{ struct_name }}`. Wire-typed
    /// payload over `SceCursor` / `SceSink`. The `Default` derive is
    /// handled separately per-instance (gated on `has_flag_default`
    /// per RFC variant-default-uniformity Atomic β), so this slot
    /// carries the category-uniform derives only.
    CodecStruct,
    /// `codec.rs.jinja2` — `pub enum {{ struct_name }}Variant`.
    /// Discriminated union over variant-arm bodies. The body field
    /// of [`Self::CodecStruct`] holds this enum, so its derive set
    /// must be the transitive prefix of CodecStruct's derives.
    CodecVariantEnum,
    /// `event_schema.rs.jinja2` — `pub struct {{ payload_struct_name }}`.
    /// W3C SCXML 5.10 event payload, equivalent wire-typed role.
    EventSchemaPayload,
    /// `enum.rs.jinja2` — `pub enum {{ enum_name }}`. Repr-tagged
    /// C-like enum (no payload arms), so `Copy` + `Eq` are natural.
    ForgeEnum,
    /// `bounded_collection.rs.jinja2` — `pub struct {{ pascal }}Handle(u32)`.
    /// Packed slot+generation newtype, used as map key (`Hash`) and
    /// value-copied across the slot-table API.
    BoundedCollectionHandle,
    /// `bounded_collection.rs.jinja2` — `pub struct {{ pascal }}OverflowError`.
    /// Zero-sized unit error returned by `insert()`.
    BoundedCollectionOverflowError,
    /// `link_bus.rs.jinja2` — `pub enum LinkBusEvent`. Outbound
    /// envelope enum carrying `Vec<u8>` per declared `<sce:link>`.
    LinkBusEvent,
}

impl RustDeriveCategory {
    /// SSOT trait list for this category. The returned slice contains
    /// the verbatim derive arguments — callers wrap with
    /// `#[derive(...)]` via [`Self::derives_attr`] for jinja injection.
    ///
    /// The wire-typed payload trio (`Debug`, `Clone`, `PartialEq`) is
    /// shared by [`Self::CodecStruct`], [`Self::CodecVariantEnum`],
    /// and [`Self::EventSchemaPayload`]: locked at this single site so
    /// the three stay byte-equivalent. [`Self::CodecStruct`] is
    /// load-bearing for downstream consumers that wrap codec types in
    /// their own `#[derive(Debug)]` enums (watching-zenoh
    /// `wz-session-core::NetworkMessage`, `DriverLoopOutcome`); a
    /// missing `Debug` here forces every such wrap site to write a
    /// manual `core::fmt::Debug` impl that recurses opaquely.
    pub fn derives(self) -> &'static [&'static str] {
        match self {
            Self::CodecStruct => &["Debug", "Clone", "PartialEq"],
            Self::CodecVariantEnum => &["Debug", "Clone", "PartialEq"],
            Self::EventSchemaPayload => &["Debug", "Clone", "PartialEq"],
            Self::ForgeEnum => &["Debug", "Clone", "Copy", "PartialEq", "Eq"],
            Self::BoundedCollectionHandle => &["Clone", "Copy", "PartialEq", "Eq", "Debug", "Hash"],
            Self::BoundedCollectionOverflowError => &["Clone", "Copy", "PartialEq", "Eq", "Debug"],
            Self::LinkBusEvent => &["Debug", "Clone"],
        }
    }

    /// Render-ready `#[derive(...)]` attribute line for jinja
    /// injection. Empty trait lists return the empty string so a
    /// future zero-derive category renders no attribute (no category
    /// uses this today, but the shape stays uniform).
    pub fn derives_attr(self) -> String {
        let traits = self.derives();
        if traits.is_empty() {
            String::new()
        } else {
            format!("#[derive({})]", traits.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_typed_payload_categories_share_baseline() {
        // CodecStruct, CodecVariantEnum, and EventSchemaPayload all
        // model plain wire-typed data — locking them to identical
        // derives at the SSOT prevents the three from drifting apart
        // under future policy edits.
        let baseline = RustDeriveCategory::CodecStruct.derives();
        assert_eq!(baseline, RustDeriveCategory::CodecVariantEnum.derives());
        assert_eq!(baseline, RustDeriveCategory::EventSchemaPayload.derives());
    }

    #[test]
    fn codec_struct_includes_debug() {
        // watching-zenoh consumer signal (2026-05-28): downstream
        // crates wrap codec types in `#[derive(Debug)]` enums, so
        // CodecStruct must carry Debug.
        assert!(RustDeriveCategory::CodecStruct.derives().contains(&"Debug"));
    }

    #[test]
    fn codec_variant_enum_includes_debug() {
        // CodecStruct's `body: NameVariant` field forces the variant
        // enum to satisfy the same derive set transitively.
        assert!(RustDeriveCategory::CodecVariantEnum
            .derives()
            .contains(&"Debug"));
    }

    #[test]
    fn derives_attr_wraps_in_derive_macro() {
        assert_eq!(
            RustDeriveCategory::CodecStruct.derives_attr(),
            "#[derive(Debug, Clone, PartialEq)]"
        );
        assert_eq!(
            RustDeriveCategory::ForgeEnum.derives_attr(),
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]"
        );
        assert_eq!(
            RustDeriveCategory::BoundedCollectionHandle.derives_attr(),
            "#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]"
        );
    }

    #[test]
    fn every_category_emits_at_least_debug() {
        // Every wire-typed Rust struct/enum SCE forge emits should be
        // diagnostable via `{:?}`. If a future category genuinely
        // can't derive Debug (e.g. it owns a non-Debug FFI handle),
        // delete this test together with the new variant in the same
        // commit so the divergence is reviewed explicitly.
        for cat in [
            RustDeriveCategory::CodecStruct,
            RustDeriveCategory::CodecVariantEnum,
            RustDeriveCategory::EventSchemaPayload,
            RustDeriveCategory::ForgeEnum,
            RustDeriveCategory::BoundedCollectionHandle,
            RustDeriveCategory::BoundedCollectionOverflowError,
            RustDeriveCategory::LinkBusEvent,
        ] {
            assert!(
                cat.derives().contains(&"Debug"),
                "category {cat:?} missing Debug — see test docstring"
            );
        }
    }
}
