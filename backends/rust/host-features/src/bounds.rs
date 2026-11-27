// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The named bounds of the records a host judges a request and its answer by
//! (docs/SCE_HOST_FEATURES.md, sections 10 to 12; docs/adr/0109, D17).
//!
//! Each is a bound the library's records name (`bytes[cose_payload=4096]`),
//! which a build moves with `sce-codegen --limit name=N`. The machine a build
//! writes declares each bound it names as a constant of the number the build
//! resolved it to (`<Machine>Limits::COSE_PAYLOAD`; docs/SCE_ACCEPTED_SUBSET.md,
//! "A bound is read by its name"); a host is given those numbers
//! ([`crate::ReferenceHost::set_named_bounds`]), so the request it accepts and
//! the answer it gives are the ones the machine's records hold. A host given
//! none holds the library's numbers ([`NamedBounds::LIBRARY`]), the bounds of
//! a build given no limit.

/// The numbers of the named bounds a host serves its operations at, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedBounds {
    /// `cose_payload`: what `sce-host-cose-sign1` signs and
    /// `sce-host-cose-verify1` answers.
    pub cose_payload: usize,
    /// `cose_signed`: the object `sce-host-cose-sign1` makes and
    /// `sce-host-cose-verify1` reads.
    pub cose_signed: usize,
    /// `cose_plaintext`: what `sce-host-cose-encrypt0` seals and
    /// `sce-host-cose-decrypt0` answers.
    pub cose_plaintext: usize,
    /// `cose_sealed`: the object `sce-host-cose-encrypt0` makes and
    /// `sce-host-cose-decrypt0` reads.
    pub cose_sealed: usize,
    /// `mac_message`: the message `sce-host-mac` tags and
    /// `sce-host-mac-verify` verifies.
    pub mac_message: usize,
    /// `random_bytes`: the most bytes `sce-host-random-bytes` draws.
    pub random_bytes: usize,
    /// `journal_page`: the most entries `sce-host-journal-read` answers in a
    /// page (docs/adr/0114, D5).
    pub journal_page: usize,
    /// `journal_page_bytes`: the most bytes the deterministic CBOR of a page
    /// `sce-host-journal-read` answers holds.
    pub journal_page_bytes: usize,
}

impl NamedBounds {
    /// The numbers the library's records write, which a build given no limit
    /// keeps.
    pub const LIBRARY: Self = Self {
        cose_payload: 4096,
        cose_signed: 4352,
        cose_plaintext: 4352,
        cose_sealed: 4480,
        mac_message: 4096,
        random_bytes: 256,
        journal_page: 64,
        journal_page_bytes: 65536,
    };
}

impl Default for NamedBounds {
    fn default() -> Self {
        Self::LIBRARY
    }
}
