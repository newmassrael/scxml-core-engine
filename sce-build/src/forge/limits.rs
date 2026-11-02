// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! SCE Forge — runtime cap constants for bytes-typed slots.
//!
//! Single source of truth (Rust side) for the default `bytes` slot
//! capacity. The C mirror lives at
//! `backends/c/forge-runtime/include/sce/forge/limits.h`; the lockstep test
//! in this module reads the C header at compile time and asserts both
//! constants resolve to the same value.

/// Default capacity for `bytes`-typed Forge slots when the SCXML author
/// has not declared a per-slot `sce:max-size` annotation. Aligned with
/// `SCE_FORGE_BYTES_DEFAULT_MAX` in
/// `backends/c/forge-runtime/include/sce/forge/limits.h`.
pub const BYTES_DEFAULT_MAX: u32 = 256;

/// Resolve a per-slot cap: if the SCXML author declared
/// `sce:max-size="N"`, use `N`; otherwise fall back to
/// [`BYTES_DEFAULT_MAX`]. A procedure's or a schema's slot only: a codec
/// field's bound is its protocol's, and no default stands in for it
/// (docs/adr/0077 §7, `CodecField::bound`).
pub fn resolve_bytes_max(annotation: Option<u32>) -> u32 {
    annotation.unwrap_or(BYTES_DEFAULT_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lockstep test: parse the C header at compile time and assert the
    /// `SCE_FORGE_BYTES_DEFAULT_MAX` macro value matches the Rust
    /// `BYTES_DEFAULT_MAX` constant. A mismatch (e.g. one side bumped
    /// without the other) trips this test before any backend can drift.
    #[test]
    fn c_header_matches_rust_constant() {
        const C_HEADER: &str =
            include_str!("../../../backends/c/forge-runtime/include/sce/forge/limits.h");

        let line = C_HEADER
            .lines()
            .find(|l| l.contains("SCE_FORGE_BYTES_DEFAULT_MAX"))
            .filter(|l| l.trim_start().starts_with("#define"))
            .expect("limits.h must define SCE_FORGE_BYTES_DEFAULT_MAX");

        // "#define SCE_FORGE_BYTES_DEFAULT_MAX 256u" → strip the `u`
        // suffix and parse. Anything other than the documented form is
        // a drift signal even if the numeric value happens to match.
        let token = line
            .split_whitespace()
            .nth(2)
            .expect("expected #define <name> <value>");
        let trimmed = token.trim_end_matches('u');
        let c_value: u32 = trimmed
            .parse()
            .expect("limits.h SCE_FORGE_BYTES_DEFAULT_MAX must be a u32 literal");

        assert_eq!(
            c_value, BYTES_DEFAULT_MAX,
            "C header SCE_FORGE_BYTES_DEFAULT_MAX = {} disagrees with \
             Rust BYTES_DEFAULT_MAX = {}; one side was edited without \
             the other",
            c_value, BYTES_DEFAULT_MAX,
        );
    }

    #[test]
    fn resolve_uses_annotation_when_present() {
        assert_eq!(resolve_bytes_max(Some(64)), 64);
    }

    #[test]
    fn resolve_falls_back_to_default_when_absent() {
        assert_eq!(resolve_bytes_max(None), BYTES_DEFAULT_MAX);
        assert_eq!(resolve_bytes_max(None), 256);
    }
}
