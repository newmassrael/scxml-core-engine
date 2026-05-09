// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// `<sce:extern>` whitelisted intrinsic registry — watching-zenoh RFC §5.I
// (lines 1691-1924), Atomic A. Closed table that mirrors the spec's
// concrete v1 whitelist (lines 1717-1750):
//
//   - Atomics, per-width × per-ordering — 90 entries
//   - Fences (acquire/release/acq_rel/seq_cst + compiler_barrier + dma_fence) — 6 entries
//   - Cache maintenance (clean / invalidate / clean_invalidate by addr) — 3 entries
//   - Interrupt control (irq_save / irq_restore) — 2 entries
//
// 101 baseline symbols. Plugin-extension symbols (deploy.yaml
// `extern_symbols.target_plugin: <path>`) are out of scope here —
// Atomic B will compose the plugin file's additions on top of this
// baseline. Q-Call-1 (a) lock: `pub const` Rust slice over YAML data,
// mirroring the `ALL_DIAGNOSTIC_CODES` drift-guard pattern.
//
// Naming + signatures are spec-verbatim per
// `feedback_spec_mirror_parity.md`; if a future spec edit renames a
// symbol or shifts a signature, the change lands here as a single
// edit and downstream `<sce:extern>` rejection messages update with no
// agent-paraphrase drift.

/// Foreign function ABI a `<sce:extern>` declaration commits to.
/// Atomic A admits only the two ABIs the spec example calls out
/// (line 1703: `abi="c"`); future axes (e.g. `system`) ride a schema
/// extension, not a shape change here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abi {
    C,
    Rust,
}

impl Abi {
    /// Authoring-side string parsed from the `abi` attribute. Returns
    /// `None` when the value is outside the closed two-element set.
    pub fn from_attr(s: &str) -> Option<Self> {
        match s {
            "c" => Some(Abi::C),
            "rust" => Some(Abi::Rust),
            _ => None,
        }
    }

    /// Canonical wire-form spelling. Used in diagnostic messages
    /// (`extern/abi-mismatch` `expected` field) so authors see the
    /// exact attribute string the registry stores.
    pub fn as_attr(self) -> &'static str {
        match self {
            Abi::C => "c",
            Abi::Rust => "rust",
        }
    }
}

/// Memory-ordering tag attached to atomic-family symbols. Names the
/// suffix the symbol's `name` ends in; a registry entry whose
/// `memory_ordering` is `Some(_)` always carries that suffix in its
/// `name` (verified at compile time by [`registry_atomic_suffixes_match`]
/// drift guard below).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryOrdering {
    Acquire,
    Release,
    AcqRel,
    Relaxed,
    SeqCst,
}

impl MemoryOrdering {
    /// Lowercase suffix as it appears in the symbol name
    /// (`sce_atomic_load_<suffix>_<width>` for load/store families,
    /// `sce_atomic_<op>_<suffix>_<width>` for fetch / cas families).
    pub fn suffix(self) -> &'static str {
        match self {
            MemoryOrdering::Acquire => "acquire",
            MemoryOrdering::Release => "release",
            MemoryOrdering::AcqRel => "acq_rel",
            MemoryOrdering::Relaxed => "relaxed",
            MemoryOrdering::SeqCst => "seq_cst",
        }
    }
}

/// Integer-width tag attached to atomic-family symbols. Spec line
/// 1717: "per-width: u8, u16, u32, u64, usize".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    U8,
    U16,
    U32,
    U64,
    USize,
}

impl Width {
    /// Lowercase suffix as it appears in the symbol name's tail.
    pub fn suffix(self) -> &'static str {
        match self {
            Width::U8 => "u8",
            Width::U16 => "u16",
            Width::U32 => "u32",
            Width::U64 => "u64",
            Width::USize => "usize",
        }
    }
}

/// Single registry entry — one `<sce:extern>` declaration shape that
/// SCE accepts at parse time.
#[derive(Debug, Clone, Copy)]
pub struct Symbol {
    /// Canonical symbol name (e.g. `sce_atomic_load_acquire_u32`).
    /// Lookup key against `<sce:extern name="...">`.
    pub name: &'static str,
    /// Canonical signature — compared exactly against
    /// `<sce:extern sig="...">`. Any drift surfaces as
    /// `extern/signature-mismatch`.
    pub sig: &'static str,
    /// Required ABI. Q-Call-3: per-symbol — atomics + fences + cache
    /// maintenance + IRQ ride `Abi::C` in v1 because the underlying
    /// runtime crate (`sce_intrinsics_runtime`) exposes them through
    /// the C ABI for stable cross-language linkage on MCU targets.
    pub abi: Abi,
    /// Crate that provides the symbol implementation. Defaults to
    /// `sce_intrinsics_runtime` for the SCE-shipped baseline; plugin
    /// extensions (Atomic B) override per-symbol.
    pub crate_name: &'static str,
    /// Free-form tag describing the symbol's purpose (Q-Call-3
    /// optional field). Surfaced in diagnostic messages for repair
    /// guidance ("you tried to use an atomic intrinsic — did you mean
    /// `sce_atomic_load_acquire_u32`?").
    pub purpose: &'static str,
    /// Memory-ordering tag for atomic-family entries; `None` for
    /// non-atomic entries (cache maintenance, IRQ control).
    pub memory_ordering: Option<MemoryOrdering>,
    /// Integer-width tag for atomic-family entries; `None` for
    /// non-atomic entries.
    pub width: Option<Width>,
    /// Spec lines 1742-1744: cache-maintenance entries MUST round
    /// `start`/`len` arguments to `platform.dcache_line_size`. Out-of-
    /// line spans trigger `mem/cache-line-alignment`. Entry's value
    /// names the minimum alignment in bytes; `None` for non-cache
    /// entries.
    pub min_dcache_alignment: Option<u32>,
}

// ── Construction helpers — keep `BASELINE_SYMBOLS` body compact ──

const fn atomic_load(name: &'static str, sig: &'static str, ord: MemoryOrdering, w: Width) -> Symbol {
    Symbol {
        name,
        sig,
        abi: Abi::C,
        crate_name: "sce_intrinsics_runtime",
        purpose: "atomic-load",
        memory_ordering: Some(ord),
        width: Some(w),
        min_dcache_alignment: None,
    }
}

const fn atomic_store(name: &'static str, sig: &'static str, ord: MemoryOrdering, w: Width) -> Symbol {
    Symbol {
        name,
        sig,
        abi: Abi::C,
        crate_name: "sce_intrinsics_runtime",
        purpose: "atomic-store",
        memory_ordering: Some(ord),
        width: Some(w),
        min_dcache_alignment: None,
    }
}

const fn atomic_cas(name: &'static str, sig: &'static str, ord: MemoryOrdering, w: Width, purpose: &'static str) -> Symbol {
    Symbol {
        name,
        sig,
        abi: Abi::C,
        crate_name: "sce_intrinsics_runtime",
        purpose,
        memory_ordering: Some(ord),
        width: Some(w),
        min_dcache_alignment: None,
    }
}

const fn atomic_fetch(name: &'static str, sig: &'static str, ord: MemoryOrdering, w: Width, purpose: &'static str) -> Symbol {
    Symbol {
        name,
        sig,
        abi: Abi::C,
        crate_name: "sce_intrinsics_runtime",
        purpose,
        memory_ordering: Some(ord),
        width: Some(w),
        min_dcache_alignment: None,
    }
}

const fn fence(name: &'static str, sig: &'static str, ord: Option<MemoryOrdering>, purpose: &'static str) -> Symbol {
    Symbol {
        name,
        sig,
        abi: Abi::C,
        crate_name: "sce_intrinsics_runtime",
        purpose,
        memory_ordering: ord,
        width: None,
        min_dcache_alignment: None,
    }
}

const fn cache(name: &'static str, sig: &'static str, purpose: &'static str) -> Symbol {
    Symbol {
        name,
        sig,
        abi: Abi::C,
        crate_name: "sce_intrinsics_runtime",
        purpose,
        memory_ordering: None,
        width: None,
        // Spec lines 1742-1744 — minimum is the platform's
        // `dcache_line_size`; the registry stores the smallest
        // ARMv7-M cortex-M7 line size as the conservative floor.
        // `mem/cache-line-alignment` checks the actual alignment
        // against the deploy-resolved line size at validate time.
        min_dcache_alignment: Some(32),
    }
}

const fn irq(name: &'static str, sig: &'static str, purpose: &'static str) -> Symbol {
    Symbol {
        name,
        sig,
        abi: Abi::C,
        crate_name: "sce_intrinsics_runtime",
        purpose,
        memory_ordering: None,
        width: None,
        min_dcache_alignment: None,
    }
}

// ── Spec-verbatim baseline (§5.I lines 1717-1750) ──────────────

use MemoryOrdering::*;
use Width::*;

/// Baseline whitelist — 101 symbols. Source of truth: watching-zenoh
/// RFC §5.I lines 1717-1750. Per-width × per-ordering combinations
/// expanded inline so each row matches its spec line one-to-one.
///
/// Drift guards (test module below) verify entry count, name-suffix
/// alignment with `memory_ordering` + `width`, and unique names.
pub const BASELINE_SYMBOLS: &[Symbol] = &[
    // ── Atomic load (spec line 1719): per-{acquire, relaxed} × 5 widths ──
    atomic_load("sce_atomic_load_acquire_u8",    "(*const u8) -> u8",       Acquire, U8),
    atomic_load("sce_atomic_load_acquire_u16",   "(*const u16) -> u16",     Acquire, U16),
    atomic_load("sce_atomic_load_acquire_u32",   "(*const u32) -> u32",     Acquire, U32),
    atomic_load("sce_atomic_load_acquire_u64",   "(*const u64) -> u64",     Acquire, U64),
    atomic_load("sce_atomic_load_acquire_usize", "(*const usize) -> usize", Acquire, USize),
    atomic_load("sce_atomic_load_relaxed_u8",    "(*const u8) -> u8",       Relaxed, U8),
    atomic_load("sce_atomic_load_relaxed_u16",   "(*const u16) -> u16",     Relaxed, U16),
    atomic_load("sce_atomic_load_relaxed_u32",   "(*const u32) -> u32",     Relaxed, U32),
    atomic_load("sce_atomic_load_relaxed_u64",   "(*const u64) -> u64",     Relaxed, U64),
    atomic_load("sce_atomic_load_relaxed_usize", "(*const usize) -> usize", Relaxed, USize),
    // ── Atomic store (line 1720): per-{release, relaxed} × 5 widths ──
    atomic_store("sce_atomic_store_release_u8",    "(*mut u8, u8)",       Release, U8),
    atomic_store("sce_atomic_store_release_u16",   "(*mut u16, u16)",     Release, U16),
    atomic_store("sce_atomic_store_release_u32",   "(*mut u32, u32)",     Release, U32),
    atomic_store("sce_atomic_store_release_u64",   "(*mut u64, u64)",     Release, U64),
    atomic_store("sce_atomic_store_release_usize", "(*mut usize, usize)", Release, USize),
    atomic_store("sce_atomic_store_relaxed_u8",    "(*mut u8, u8)",       Relaxed, U8),
    atomic_store("sce_atomic_store_relaxed_u16",   "(*mut u16, u16)",     Relaxed, U16),
    atomic_store("sce_atomic_store_relaxed_u32",   "(*mut u32, u32)",     Relaxed, U32),
    atomic_store("sce_atomic_store_relaxed_u64",   "(*mut u64, u64)",     Relaxed, U64),
    atomic_store("sce_atomic_store_relaxed_usize", "(*mut usize, usize)", Relaxed, USize),
    // ── CAS-weak (line 1721): per-{acq_rel, release, relaxed} × 5 widths ──
    //    "returns old value" per spec comment.
    atomic_cas("sce_atomic_cas_weak_acq_rel_u8",    "(*mut u8, u8, u8) -> u8",          AcqRel,  U8,    "atomic-cas-weak"),
    atomic_cas("sce_atomic_cas_weak_acq_rel_u16",   "(*mut u16, u16, u16) -> u16",      AcqRel,  U16,   "atomic-cas-weak"),
    atomic_cas("sce_atomic_cas_weak_acq_rel_u32",   "(*mut u32, u32, u32) -> u32",      AcqRel,  U32,   "atomic-cas-weak"),
    atomic_cas("sce_atomic_cas_weak_acq_rel_u64",   "(*mut u64, u64, u64) -> u64",      AcqRel,  U64,   "atomic-cas-weak"),
    atomic_cas("sce_atomic_cas_weak_acq_rel_usize", "(*mut usize, usize, usize) -> usize", AcqRel, USize, "atomic-cas-weak"),
    atomic_cas("sce_atomic_cas_weak_release_u8",    "(*mut u8, u8, u8) -> u8",          Release, U8,    "atomic-cas-weak"),
    atomic_cas("sce_atomic_cas_weak_release_u16",   "(*mut u16, u16, u16) -> u16",      Release, U16,   "atomic-cas-weak"),
    atomic_cas("sce_atomic_cas_weak_release_u32",   "(*mut u32, u32, u32) -> u32",      Release, U32,   "atomic-cas-weak"),
    atomic_cas("sce_atomic_cas_weak_release_u64",   "(*mut u64, u64, u64) -> u64",      Release, U64,   "atomic-cas-weak"),
    atomic_cas("sce_atomic_cas_weak_release_usize", "(*mut usize, usize, usize) -> usize", Release, USize, "atomic-cas-weak"),
    atomic_cas("sce_atomic_cas_weak_relaxed_u8",    "(*mut u8, u8, u8) -> u8",          Relaxed, U8,    "atomic-cas-weak"),
    atomic_cas("sce_atomic_cas_weak_relaxed_u16",   "(*mut u16, u16, u16) -> u16",      Relaxed, U16,   "atomic-cas-weak"),
    atomic_cas("sce_atomic_cas_weak_relaxed_u32",   "(*mut u32, u32, u32) -> u32",      Relaxed, U32,   "atomic-cas-weak"),
    atomic_cas("sce_atomic_cas_weak_relaxed_u64",   "(*mut u64, u64, u64) -> u64",      Relaxed, U64,   "atomic-cas-weak"),
    atomic_cas("sce_atomic_cas_weak_relaxed_usize", "(*mut usize, usize, usize) -> usize", Relaxed, USize, "atomic-cas-weak"),
    // ── CAS-strong (line 1722): per-{acq_rel, release, relaxed} × 5 widths ──
    atomic_cas("sce_atomic_cas_strong_acq_rel_u8",    "(*mut u8, u8, u8) -> u8",          AcqRel,  U8,    "atomic-cas-strong"),
    atomic_cas("sce_atomic_cas_strong_acq_rel_u16",   "(*mut u16, u16, u16) -> u16",      AcqRel,  U16,   "atomic-cas-strong"),
    atomic_cas("sce_atomic_cas_strong_acq_rel_u32",   "(*mut u32, u32, u32) -> u32",      AcqRel,  U32,   "atomic-cas-strong"),
    atomic_cas("sce_atomic_cas_strong_acq_rel_u64",   "(*mut u64, u64, u64) -> u64",      AcqRel,  U64,   "atomic-cas-strong"),
    atomic_cas("sce_atomic_cas_strong_acq_rel_usize", "(*mut usize, usize, usize) -> usize", AcqRel, USize, "atomic-cas-strong"),
    atomic_cas("sce_atomic_cas_strong_release_u8",    "(*mut u8, u8, u8) -> u8",          Release, U8,    "atomic-cas-strong"),
    atomic_cas("sce_atomic_cas_strong_release_u16",   "(*mut u16, u16, u16) -> u16",      Release, U16,   "atomic-cas-strong"),
    atomic_cas("sce_atomic_cas_strong_release_u32",   "(*mut u32, u32, u32) -> u32",      Release, U32,   "atomic-cas-strong"),
    atomic_cas("sce_atomic_cas_strong_release_u64",   "(*mut u64, u64, u64) -> u64",      Release, U64,   "atomic-cas-strong"),
    atomic_cas("sce_atomic_cas_strong_release_usize", "(*mut usize, usize, usize) -> usize", Release, USize, "atomic-cas-strong"),
    atomic_cas("sce_atomic_cas_strong_relaxed_u8",    "(*mut u8, u8, u8) -> u8",          Relaxed, U8,    "atomic-cas-strong"),
    atomic_cas("sce_atomic_cas_strong_relaxed_u16",   "(*mut u16, u16, u16) -> u16",      Relaxed, U16,   "atomic-cas-strong"),
    atomic_cas("sce_atomic_cas_strong_relaxed_u32",   "(*mut u32, u32, u32) -> u32",      Relaxed, U32,   "atomic-cas-strong"),
    atomic_cas("sce_atomic_cas_strong_relaxed_u64",   "(*mut u64, u64, u64) -> u64",      Relaxed, U64,   "atomic-cas-strong"),
    atomic_cas("sce_atomic_cas_strong_relaxed_usize", "(*mut usize, usize, usize) -> usize", Relaxed, USize, "atomic-cas-strong"),
    // ── fetch_add (line 1723): per-{acq_rel, relaxed} × 5 widths ──
    atomic_fetch("sce_atomic_fetch_add_acq_rel_u8",    "(*mut u8, u8) -> u8",          AcqRel,  U8,    "atomic-fetch-add"),
    atomic_fetch("sce_atomic_fetch_add_acq_rel_u16",   "(*mut u16, u16) -> u16",       AcqRel,  U16,   "atomic-fetch-add"),
    atomic_fetch("sce_atomic_fetch_add_acq_rel_u32",   "(*mut u32, u32) -> u32",       AcqRel,  U32,   "atomic-fetch-add"),
    atomic_fetch("sce_atomic_fetch_add_acq_rel_u64",   "(*mut u64, u64) -> u64",       AcqRel,  U64,   "atomic-fetch-add"),
    atomic_fetch("sce_atomic_fetch_add_acq_rel_usize", "(*mut usize, usize) -> usize", AcqRel, USize, "atomic-fetch-add"),
    atomic_fetch("sce_atomic_fetch_add_relaxed_u8",    "(*mut u8, u8) -> u8",          Relaxed, U8,    "atomic-fetch-add"),
    atomic_fetch("sce_atomic_fetch_add_relaxed_u16",   "(*mut u16, u16) -> u16",       Relaxed, U16,   "atomic-fetch-add"),
    atomic_fetch("sce_atomic_fetch_add_relaxed_u32",   "(*mut u32, u32) -> u32",       Relaxed, U32,   "atomic-fetch-add"),
    atomic_fetch("sce_atomic_fetch_add_relaxed_u64",   "(*mut u64, u64) -> u64",       Relaxed, U64,   "atomic-fetch-add"),
    atomic_fetch("sce_atomic_fetch_add_relaxed_usize", "(*mut usize, usize) -> usize", Relaxed, USize, "atomic-fetch-add"),
    // ── fetch_sub (line 1724): per-{acq_rel, relaxed} × 5 widths ──
    atomic_fetch("sce_atomic_fetch_sub_acq_rel_u8",    "(*mut u8, u8) -> u8",          AcqRel,  U8,    "atomic-fetch-sub"),
    atomic_fetch("sce_atomic_fetch_sub_acq_rel_u16",   "(*mut u16, u16) -> u16",       AcqRel,  U16,   "atomic-fetch-sub"),
    atomic_fetch("sce_atomic_fetch_sub_acq_rel_u32",   "(*mut u32, u32) -> u32",       AcqRel,  U32,   "atomic-fetch-sub"),
    atomic_fetch("sce_atomic_fetch_sub_acq_rel_u64",   "(*mut u64, u64) -> u64",       AcqRel,  U64,   "atomic-fetch-sub"),
    atomic_fetch("sce_atomic_fetch_sub_acq_rel_usize", "(*mut usize, usize) -> usize", AcqRel, USize, "atomic-fetch-sub"),
    atomic_fetch("sce_atomic_fetch_sub_relaxed_u8",    "(*mut u8, u8) -> u8",          Relaxed, U8,    "atomic-fetch-sub"),
    atomic_fetch("sce_atomic_fetch_sub_relaxed_u16",   "(*mut u16, u16) -> u16",       Relaxed, U16,   "atomic-fetch-sub"),
    atomic_fetch("sce_atomic_fetch_sub_relaxed_u32",   "(*mut u32, u32) -> u32",       Relaxed, U32,   "atomic-fetch-sub"),
    atomic_fetch("sce_atomic_fetch_sub_relaxed_u64",   "(*mut u64, u64) -> u64",       Relaxed, U64,   "atomic-fetch-sub"),
    atomic_fetch("sce_atomic_fetch_sub_relaxed_usize", "(*mut usize, usize) -> usize", Relaxed, USize, "atomic-fetch-sub"),
    // ── fetch_or (line 1725): per-{acq_rel, relaxed} × 5 widths ──
    atomic_fetch("sce_atomic_fetch_or_acq_rel_u8",    "(*mut u8, u8) -> u8",          AcqRel,  U8,    "atomic-fetch-or"),
    atomic_fetch("sce_atomic_fetch_or_acq_rel_u16",   "(*mut u16, u16) -> u16",       AcqRel,  U16,   "atomic-fetch-or"),
    atomic_fetch("sce_atomic_fetch_or_acq_rel_u32",   "(*mut u32, u32) -> u32",       AcqRel,  U32,   "atomic-fetch-or"),
    atomic_fetch("sce_atomic_fetch_or_acq_rel_u64",   "(*mut u64, u64) -> u64",       AcqRel,  U64,   "atomic-fetch-or"),
    atomic_fetch("sce_atomic_fetch_or_acq_rel_usize", "(*mut usize, usize) -> usize", AcqRel, USize, "atomic-fetch-or"),
    atomic_fetch("sce_atomic_fetch_or_relaxed_u8",    "(*mut u8, u8) -> u8",          Relaxed, U8,    "atomic-fetch-or"),
    atomic_fetch("sce_atomic_fetch_or_relaxed_u16",   "(*mut u16, u16) -> u16",       Relaxed, U16,   "atomic-fetch-or"),
    atomic_fetch("sce_atomic_fetch_or_relaxed_u32",   "(*mut u32, u32) -> u32",       Relaxed, U32,   "atomic-fetch-or"),
    atomic_fetch("sce_atomic_fetch_or_relaxed_u64",   "(*mut u64, u64) -> u64",       Relaxed, U64,   "atomic-fetch-or"),
    atomic_fetch("sce_atomic_fetch_or_relaxed_usize", "(*mut usize, usize) -> usize", Relaxed, USize, "atomic-fetch-or"),
    // ── fetch_and (line 1726): per-{acq_rel, relaxed} × 5 widths ──
    atomic_fetch("sce_atomic_fetch_and_acq_rel_u8",    "(*mut u8, u8) -> u8",          AcqRel,  U8,    "atomic-fetch-and"),
    atomic_fetch("sce_atomic_fetch_and_acq_rel_u16",   "(*mut u16, u16) -> u16",       AcqRel,  U16,   "atomic-fetch-and"),
    atomic_fetch("sce_atomic_fetch_and_acq_rel_u32",   "(*mut u32, u32) -> u32",       AcqRel,  U32,   "atomic-fetch-and"),
    atomic_fetch("sce_atomic_fetch_and_acq_rel_u64",   "(*mut u64, u64) -> u64",       AcqRel,  U64,   "atomic-fetch-and"),
    atomic_fetch("sce_atomic_fetch_and_acq_rel_usize", "(*mut usize, usize) -> usize", AcqRel, USize, "atomic-fetch-and"),
    atomic_fetch("sce_atomic_fetch_and_relaxed_u8",    "(*mut u8, u8) -> u8",          Relaxed, U8,    "atomic-fetch-and"),
    atomic_fetch("sce_atomic_fetch_and_relaxed_u16",   "(*mut u16, u16) -> u16",       Relaxed, U16,   "atomic-fetch-and"),
    atomic_fetch("sce_atomic_fetch_and_relaxed_u32",   "(*mut u32, u32) -> u32",       Relaxed, U32,   "atomic-fetch-and"),
    atomic_fetch("sce_atomic_fetch_and_relaxed_u64",   "(*mut u64, u64) -> u64",       Relaxed, U64,   "atomic-fetch-and"),
    atomic_fetch("sce_atomic_fetch_and_relaxed_usize", "(*mut usize, usize) -> usize", Relaxed, USize, "atomic-fetch-and"),
    // ── Fences (lines 1729-1733): 4 atomic-fence × ord + compiler_barrier + dma_fence ──
    fence("sce_atomic_fence_acquire", "()", Some(Acquire), "atomic-fence"),
    fence("sce_atomic_fence_release", "()", Some(Release), "atomic-fence"),
    fence("sce_atomic_fence_acq_rel", "()", Some(AcqRel),  "atomic-fence"),
    fence("sce_atomic_fence_seq_cst", "()", Some(SeqCst),  "atomic-fence"),
    fence("sce_compiler_barrier", "()", None, "compiler-barrier"),
    fence("sce_dma_fence",        "()", None, "dma-fence"),
    // ── Cache maintenance (lines 1736-1740): 3 entries ──
    cache("sce_dcache_clean_by_addr",            "(*const c_void, usize)", "cache-clean"),
    cache("sce_dcache_invalidate_by_addr",       "(*mut c_void, usize)",   "cache-invalidate"),
    cache("sce_dcache_clean_invalidate_by_addr", "(*mut c_void, usize)",   "cache-clean-invalidate"),
    // ── Interrupt control (lines 1748-1749): 2 entries ──
    irq("sce_irq_save",    "() -> irq_state_t", "irq-save"),
    irq("sce_irq_restore", "(irq_state_t)",     "irq-restore"),
];

/// Closed-set lookup over [`BASELINE_SYMBOLS`]. Returns the registry
/// entry whose `name` matches `name` exactly. Used by the validator's
/// parse-time rejection path.
pub fn lookup_symbol(name: &str) -> Option<&'static Symbol> {
    BASELINE_SYMBOLS.iter().find(|s| s.name == name)
}

/// Suffix-aware lookup hint for `extern/ordering-unspecified`. When
/// the author writes a base-name without an ordering suffix (e.g.
/// `sce_atomic_load`), this helper returns the list of legal
/// suffix-bearing names so the diagnostic's repair guidance can
/// surface them without requiring the validator to enumerate the
/// closed set itself.
///
/// Returns `None` when no symbol in the registry has `base_name` as a
/// strict prefix followed by `_<ord>_<width>`. That case is handled by
/// the wider `extern/symbol-not-in-whitelist` code instead.
pub fn ordering_suffix_completions(base_name: &str) -> Option<Vec<&'static str>> {
    // Atomic-family base prefixes — the only family where the spec
    // calls out `extern/ordering-unspecified` (line 1850: "atomic
    // intrinsic invoked without explicit ordering suffix").
    const ATOMIC_BASES: &[&str] = &[
        "sce_atomic_load",
        "sce_atomic_store",
        "sce_atomic_cas_weak",
        "sce_atomic_cas_strong",
        "sce_atomic_fetch_add",
        "sce_atomic_fetch_sub",
        "sce_atomic_fetch_or",
        "sce_atomic_fetch_and",
        "sce_atomic_fence",
    ];
    if !ATOMIC_BASES.iter().any(|b| *b == base_name) {
        return None;
    }
    let prefix = format!("{base_name}_");
    let mut hits: Vec<&'static str> = BASELINE_SYMBOLS
        .iter()
        .filter(|s| s.name.starts_with(&prefix) && s.memory_ordering.is_some())
        .map(|s| s.name)
        .collect();
    hits.sort_unstable();
    if hits.is_empty() {
        None
    } else {
        Some(hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Spec-verbatim entry count. 90 atomic + 6 fence + 3 cache + 2
    /// IRQ = 101. Drift here = a future spec edit reshaped the
    /// baseline; cross-check `BASELINE_SYMBOLS` against §5.I lines
    /// 1717-1750 before bumping the count.
    #[test]
    fn baseline_symbol_count_matches_spec() {
        assert_eq!(
            BASELINE_SYMBOLS.len(),
            101,
            "BASELINE_SYMBOLS drifted from spec §5.I lines 1717-1750"
        );
    }

    #[test]
    fn baseline_symbol_names_unique() {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for s in BASELINE_SYMBOLS {
            assert!(
                seen.insert(s.name),
                "duplicate symbol name in baseline: {}",
                s.name
            );
        }
    }

    #[test]
    fn lookup_symbol_finds_baseline_entries() {
        assert!(lookup_symbol("sce_atomic_load_acquire_u32").is_some());
        assert!(lookup_symbol("sce_dcache_clean_by_addr").is_some());
        assert!(lookup_symbol("sce_irq_save").is_some());
        assert!(lookup_symbol("sce_atomic_load").is_none(), "base-name without suffix must not resolve");
        assert!(lookup_symbol("not_in_registry").is_none());
    }

    /// Drift guard: every atomic-family entry's name suffix must
    /// match its `memory_ordering` + `width` tags. A future macro
    /// regression that misspells `acq_rel` as `acqrel` (or pairs
    /// `Acquire` with `_release_` text) surfaces here, not at the
    /// downstream codegen template that emits the suffix.
    #[test]
    fn registry_atomic_suffixes_match() {
        for s in BASELINE_SYMBOLS {
            if let (Some(ord), Some(w)) = (s.memory_ordering, s.width) {
                let expected = format!("_{}_{}", ord.suffix(), w.suffix());
                assert!(
                    s.name.ends_with(&expected),
                    "{}: expected to end with {expected}",
                    s.name
                );
            }
        }
    }

    #[test]
    fn ordering_suffix_completions_lists_all_load_widths() {
        let hits = ordering_suffix_completions("sce_atomic_load")
            .expect("`sce_atomic_load` is an atomic-family base");
        // 2 orderings × 5 widths = 10 hits.
        assert_eq!(hits.len(), 10, "got {hits:?}");
    }

    #[test]
    fn ordering_suffix_completions_returns_none_on_non_atomic() {
        assert!(ordering_suffix_completions("sce_dcache_clean_by_addr").is_none());
        assert!(ordering_suffix_completions("sce_irq_save").is_none());
        assert!(ordering_suffix_completions("not_a_base").is_none());
    }

    #[test]
    fn abi_round_trip() {
        assert_eq!(Abi::from_attr("c"), Some(Abi::C));
        assert_eq!(Abi::from_attr("rust"), Some(Abi::Rust));
        assert!(Abi::from_attr("system").is_none());
        assert_eq!(Abi::C.as_attr(), "c");
        assert_eq!(Abi::Rust.as_attr(), "rust");
    }
}
