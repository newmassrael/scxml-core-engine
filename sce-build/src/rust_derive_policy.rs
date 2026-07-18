// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Single source of truth for `#[derive(...)]` attributes on every
//! Rust type the codegen emits.
//!
//! Two engines emit Rust with derive attributes: the statechart
//! codegen ([`crate::generator`], `tools/codegen/templates/rust/*.rs.jinja2`)
//! and the forge codegen ([`crate::forge::generator`],
//! `tools/codegen/templates/forge/rust/*.rs.jinja2`). Before this
//! module, each template hardcoded its own derive list inline.
//! Cross-cutting policy changes (adding `Debug` everywhere, adding
//! `#[non_exhaustive]`, gating on a new cargo feature) had to be
//! applied in N places, with no compile-time check that N stayed in
//! sync. Drift was silent: a missed template still rendered and
//! compiled. This module lives at the crate root (not under `forge/`)
//! so neither engine reaches into the other's namespace for the
//! shared policy.
//!
//! The categories here cover every contract-bearing type the Rust
//! templates emit — types whose derive set is a cross-boundary
//! concern: wire-typed payloads (codec borrowed + owned, event
//! schema), repr-tagged / cross-language enums (forge enum, lookup,
//! observer, buffer-pool slot-state, bounded-collection handle), the
//! link-bus envelope, the validator result, and the consumer-injectable
//! statechart / procedure lifecycle enums. Each render function injects
//! [`derives_attr`] for its category into the jinja context under a
//! category-named key (e.g. `codec_struct_derives_attr`,
//! `state_derives_attr`), and the template renders the result verbatim
//! — templates carry no policy of their own.
//!
//! Deliberately NOT centralized: purely-internal types whose derive
//! set is a local concern with no cross-boundary contract — the
//! private `TransitionInfo` helper in `state_machine.rs.jinja2` and
//! the zero-sized `Slot<S>` phantom-marker structs in
//! `buffer_pool.rs.jinja2`. Centralizing a private helper's `Debug`
//! into a global policy would trade locality for uniformity with no
//! drift-contract to protect. Their derives stay inline by design.
//!
//! Callers may APPEND extra derives to a category without changing the
//! SSOT default via [`render_derives_attr`] (defaults + caller extras,
//! deduped). This is how a downstream consumer opts a generated enum
//! into `serde` or its own `#[derive(...)]` proc-macro without SCE
//! taking on the dependency: the extra paths are passed through
//! verbatim and resolve in the consuming crate at its compile time.
//!
//! Adding a new emitted struct/enum:
//!   1. Add a [`RustDeriveCategory`] variant with the textbook derive
//!      list for that shape (mirror existing wire-typed payload policy
//!      when the shape is plain data).
//!   2. Inject `<key>_derives_attr` into the matching render function's
//!      jinja context (Rust language arm only).
//!   3. Replace the template's hardcoded `#[derive(...)]` line with
//!      `{{ <key>_derives_attr }}`.
//!   4. Extend `sce-build/tests/rust_derive_ssot.rs` to lock the new
//!      category.

/// Category of forge-emitted Rust struct or enum, partitioned by wire
/// role rather than by template file (one template may emit multiple
/// categories, e.g. `codec.rs.jinja2` emits both [`Self::CodecStruct`]
/// and [`Self::CodecVariantEnum`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustDeriveCategory {
    /// `codec.rs.jinja2` — `pub struct {{ struct_name }}`. Wire-typed
    /// payload over `SceCursor` / `SceSink`. The `Default` derive is
    /// handled separately per-instance (gated on `has_flag_default`
    /// per the variant-default-uniformity rollup), so this slot
    /// carries the category-uniform derives only.
    CodecStruct,
    /// `codec.rs.jinja2` — `pub enum {{ struct_name }}Variant`.
    /// Discriminated union over variant-arm bodies. The body field
    /// of [`Self::CodecStruct`] holds this enum, so its derive set
    /// must be the transitive prefix of CodecStruct's derives.
    CodecVariantEnum,
    /// `event_schema.rs.jinja2` — `pub struct {{ payload_struct_name }}`.
    /// §scxml-5.10 event payload, equivalent wire-typed role.
    EventSchemaPayload,
    /// Repr-tagged C-like forge enum (no payload arms), so `Copy` +
    /// `Eq` are natural. One policy for one shape: covers every forge
    /// repr-tagged enum — `enum.rs.jinja2`'s `pub enum {{ enum_name }}`,
    /// `lookup.rs.jinja2`'s string-output enum, `observer.rs.jinja2`'s
    /// `ForgeDomainTag`, and `buffer_pool.rs.jinja2`'s `SlotState`.
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
    /// `rust/state_machine.rs.jinja2` — `pub enum {{ machine_name }}State`.
    /// Statechart lifecycle enum (W3C SCXML 3.3). Repr-untagged C-like
    /// enum used as a `HashMap` key across the generated transition
    /// tables, so `Hash` is carried in addition to the Copy-trivial
    /// set. Distinct from [`Self::ProcedureState`] precisely because of
    /// that `Hash` — the two must not be collapsed.
    StatechartState,
    /// `rust/state_machine.rs.jinja2` — `pub enum {{ machine_name }}Event`.
    /// Statechart trigger enum (W3C SCXML 3.12). Same shape as
    /// [`Self::StatechartState`].
    StatechartEvent,
    /// `forge/rust/procedure.rs.jinja2` — `pub enum State`. Forge
    /// procedure lifecycle enum. Repr-tagged C-like enum (explicit
    /// `= index` discriminants, no payload arms). Carries no `Hash`
    /// today — procedures do not key maps on state — so it stays
    /// separate from [`Self::StatechartState`].
    ProcedureState,
    /// `forge/rust/procedure.rs.jinja2` — `pub enum Event`. Forge
    /// procedure trigger enum. Same shape as [`Self::ProcedureState`].
    ProcedureEvent,
    /// `validator.rs.jinja2` — `pub struct ValidationResult`. Forge
    /// validator's pub-API result (SCE_FORGE.md §7). Not wire-typed
    /// and not `Copy` (owns a `String reason`), so `Debug` alone is
    /// the natural set; a consumer that needs more derives it locally.
    ValidatorResult,
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
            Self::StatechartState | Self::StatechartEvent => {
                &["Debug", "Clone", "Copy", "PartialEq", "Eq", "Hash"]
            }
            Self::ProcedureState | Self::ProcedureEvent => {
                &["Debug", "Clone", "Copy", "PartialEq", "Eq"]
            }
            Self::ValidatorResult => &["Debug"],
        }
    }

    /// Render-ready `#[derive(...)]` attribute line for jinja
    /// injection, defaults only. Delegates to [`render_derives_attr`]
    /// with no caller extras so the formatting stays identical between
    /// the defaults-only and extras paths.
    pub fn derives_attr(self) -> String {
        render_derives_attr(self.derives(), &[])
    }
}

/// Build a `#[derive(...)]` attribute line from a category's default
/// trait list plus caller-supplied extra derive arguments, appended
/// verbatim and deduped (first occurrence wins, preserving order).
///
/// Extras are passed through unresolved — a caller may name a
/// fully-qualified path (`serde::Serialize`, `my_crate::MyDerive`)
/// and it renders as written; resolution happens in the consuming
/// crate at its compile time, so SCE takes on no dependency for a
/// derive it merely forwards. Dedup is by exact string, so a bare
/// `Serialize` and a qualified `serde::Serialize` are treated as
/// distinct (they are, to the Rust compiler).
///
/// An empty result (empty defaults and empty extras) renders the empty
/// string so a zero-derive category emits no attribute — the shape
/// stays uniform for a future such category.
pub fn render_derives_attr(defaults: &[&str], extras: &[String]) -> String {
    let mut traits: Vec<&str> = Vec::with_capacity(defaults.len() + extras.len());
    for &d in defaults {
        if !traits.contains(&d) {
            traits.push(d);
        }
    }
    for e in extras {
        let e = e.as_str();
        if !traits.contains(&e) {
            traits.push(e);
        }
    }
    if traits.is_empty() {
        String::new()
    } else {
        format!("#[derive({})]", traits.join(", "))
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
            RustDeriveCategory::StatechartState,
            RustDeriveCategory::StatechartEvent,
            RustDeriveCategory::ProcedureState,
            RustDeriveCategory::ProcedureEvent,
            RustDeriveCategory::ValidatorResult,
        ] {
            assert!(
                cat.derives().contains(&"Debug"),
                "category {cat:?} missing Debug — see test docstring"
            );
        }
    }

    #[test]
    fn statechart_enums_carry_hash_but_procedure_enums_do_not() {
        // The two Rust enum-emitting engines emit different derive
        // sets: the statechart transition tables key HashMaps on the
        // State enum (so `Hash`), while forge procedures do not. This
        // divergence is the concrete reason the categories are distinct
        // rather than a shared repr-tagged-enum category. If a future
        // change unifies them, delete this test in the same commit.
        assert!(RustDeriveCategory::StatechartState
            .derives()
            .contains(&"Hash"));
        assert!(RustDeriveCategory::StatechartEvent
            .derives()
            .contains(&"Hash"));
        assert!(!RustDeriveCategory::ProcedureState
            .derives()
            .contains(&"Hash"));
        assert!(!RustDeriveCategory::ProcedureEvent
            .derives()
            .contains(&"Hash"));
    }

    #[test]
    fn statechart_derives_attr_matches_pre_ssot_hardcoded_line() {
        // Byte-for-byte the string the statechart template hardcoded
        // before joining the SSOT — locks the refactor as no-op.
        assert_eq!(
            RustDeriveCategory::StatechartState.derives_attr(),
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]"
        );
    }

    #[test]
    fn procedure_derives_attr_matches_pre_ssot_hardcoded_line() {
        // Byte-for-byte the string the forge procedure template
        // hardcoded before joining the SSOT.
        assert_eq!(
            RustDeriveCategory::ProcedureState.derives_attr(),
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]"
        );
    }

    #[test]
    fn validator_result_is_debug_only() {
        // `ValidationResult` owns a `String reason`, so it is not `Copy`
        // and carries `Debug` alone — a consumer needing Clone/PartialEq
        // derives them locally. Locks the minimal set.
        assert_eq!(
            RustDeriveCategory::ValidatorResult.derives_attr(),
            "#[derive(Debug)]"
        );
    }

    #[test]
    fn render_derives_attr_empty_defaults_and_extras_is_empty_string() {
        assert_eq!(render_derives_attr(&[], &[]), "");
    }

    #[test]
    fn render_derives_attr_appends_extras_after_defaults() {
        let extras = vec![
            "serde::Serialize".to_string(),
            "serde::Deserialize".to_string(),
        ];
        assert_eq!(
            render_derives_attr(RustDeriveCategory::StatechartState.derives(), &extras),
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]"
        );
    }

    #[test]
    fn render_derives_attr_dedups_extra_that_repeats_a_default() {
        // A caller re-listing a default trait is harmless — deduped,
        // first occurrence (the default) wins, order preserved.
        let extras = vec!["Debug".to_string(), "Hash".to_string()];
        assert_eq!(
            render_derives_attr(RustDeriveCategory::StatechartState.derives(), &extras),
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]"
        );
    }

    #[test]
    fn render_derives_attr_only_extras_when_defaults_empty() {
        let extras = vec!["Foo".to_string(), "Bar".to_string()];
        assert_eq!(render_derives_attr(&[], &extras), "#[derive(Foo, Bar)]");
    }
}
