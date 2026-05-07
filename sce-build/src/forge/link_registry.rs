// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Forge link registry — build-time index of named link kind artifacts
// declared in `.forge` files, consulted by SCXML cross-reference
// validators that point at those names (e.g.
// `<sce:on-sample link="X">`).
//
// Background. SCE separates link *templates* (forge `<scxml
// sce:kind="link">` documents — class enum, framer, RX/TX pool
// references, backpressure declarations) from link *consumers*
// (state-level `<sce:on-sample>` declarations that subscribe to a
// link's RX path at SCXML state-machine init time). Until this
// module landed, SCXML had no cross-schema reference into forge
// link kind — the link-kind ↔ codec / pool wiring was forge-internal,
// and SCXML validators only consulted the parsed `SCXMLModel`.
//
// Spec anchor. watching-zenoh RFC §5.E B7-η' Atomic B introduces the
// `<sce:on-sample link="X">` cross-reference resolution surface;
// callers populate this registry from every parsed `.forge` document
// in the build, then SCXML validators query `lookup(name)` to
// confirm the link kind exists. Pattern mirrors
// [`crate::forge::pool_registry::ForgePoolRegistry`] verbatim — the
// two registries deliberately stay parallel rather than merging into
// a generic artifact registry, so each consumer module owns the
// authoritative shape of its kind.

use std::collections::HashMap;

use super::model::ForgeDocument;

/// Link kinds that participate in the SCXML cross-reference surface.
/// Adding a kind here is the textbook way to extend the registry's
/// classification surface — new variants force exhaustive matches at
/// every consumer (e.g. `<sce:on-sample>` validator's "wrong-kind"
/// diagnostic), which is precisely the drift protection we want for
/// cross-schema names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeLinkKind {
    /// `<scxml sce:kind="link">` — byte-stream link endpoint
    /// (§5.C B6-α/β/γ landed). Today the only link kind a SCXML
    /// `<sce:on-sample link="X">` reference may resolve to.
    Link,
}

impl ForgeLinkKind {
    /// Slash-path label for this kind — used in diagnostic messages
    /// rather than `Debug` so the wire form stays stable across Rust
    /// edition / `Debug`-impl changes.
    pub fn as_str(&self) -> &'static str {
        match self {
            ForgeLinkKind::Link => "link",
        }
    }
}

/// Build-time index of named forge link artifacts. Populated once
/// per build by walking every parsed forge document and calling
/// [`Self::record`] (or [`Self::record_document`]); consulted by
/// SCXML validators that need to resolve a name reference (e.g.
/// `<sce:on-sample link="X">`).
///
/// Names are unique across all link kinds — duplicates are rejected
/// at registration time so `lookup` can return a single answer.
/// Forge itself already enforces unique artifact names within a
/// build, so duplicates surfacing here would indicate a producer
/// bug; the assert in `record` is a defensive belt-and-braces.
#[derive(Debug, Default)]
pub struct ForgeLinkRegistry {
    links: HashMap<String, ForgeLinkKind>,
}

impl ForgeLinkRegistry {
    /// Empty registry. Caller registers entries by walking the
    /// forge build's parsed documents.
    pub fn new() -> Self {
        Self {
            links: HashMap::new(),
        }
    }

    /// Register one link artifact. Returns `Err` with the existing
    /// kind if the name is already registered with a different kind;
    /// no-op if the same name + same kind is re-registered
    /// (idempotent for repeat calls during incremental builds).
    pub fn record(
        &mut self,
        name: impl Into<String>,
        kind: ForgeLinkKind,
    ) -> Result<(), ForgeLinkKind> {
        let name = name.into();
        match self.links.get(&name) {
            Some(existing) if *existing == kind => Ok(()),
            Some(existing) => Err(*existing),
            None => {
                self.links.insert(name, kind);
                Ok(())
            }
        }
    }

    /// Register a forge document if its kind is a link kind. No-op
    /// for non-link documents — matches the "register every parsed
    /// forge document" call-site pattern without the caller having
    /// to filter.
    pub fn record_document(&mut self, doc: &ForgeDocument) -> Result<(), ForgeLinkKind> {
        if let ForgeDocument::Link(link) = doc {
            self.record(link.name.clone(), ForgeLinkKind::Link)
        } else {
            Ok(())
        }
    }

    /// Resolve a name reference. `None` means the name is not
    /// declared as a link in any `.forge` file; consumers raise the
    /// `scxml/on-sample-link-not-declared` diagnostic.
    pub fn lookup(&self, name: &str) -> Option<ForgeLinkKind> {
        self.links.get(name).copied()
    }

    /// Sorted list of registered link names of a given kind.
    /// Diagnostics use this for `Fix::ReplaceOneOf` candidate lists
    /// so authors see legal alternatives when their reference does
    /// not resolve.
    pub fn names_of_kind(&self, kind: ForgeLinkKind) -> Vec<String> {
        let mut out: Vec<String> = self
            .links
            .iter()
            .filter(|(_, k)| **k == kind)
            .map(|(n, _)| n.clone())
            .collect();
        out.sort();
        out
    }

    /// Total count across all link kinds. Surfaced for debugging
    /// and for tests that assert registry construction observed the
    /// expected number of artifacts.
    pub fn len(&self) -> usize {
        self.links.len()
    }

    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup_link() {
        let mut reg = ForgeLinkRegistry::new();
        assert!(reg.is_empty());
        reg.record("scout_link", ForgeLinkKind::Link).unwrap();
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.lookup("scout_link"), Some(ForgeLinkKind::Link));
        assert_eq!(reg.lookup("missing"), None);
    }

    #[test]
    fn record_idempotent_on_same_kind() {
        let mut reg = ForgeLinkRegistry::new();
        reg.record("scout_link", ForgeLinkKind::Link).unwrap();
        // Second call with the same kind succeeds — incremental
        // builds re-walking parsed documents must not raise.
        reg.record("scout_link", ForgeLinkKind::Link).unwrap();
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn names_of_kind_returns_sorted() {
        let mut reg = ForgeLinkRegistry::new();
        reg.record("zeta_link", ForgeLinkKind::Link).unwrap();
        reg.record("alpha_link", ForgeLinkKind::Link).unwrap();
        reg.record("middle_link", ForgeLinkKind::Link).unwrap();
        assert_eq!(
            reg.names_of_kind(ForgeLinkKind::Link),
            vec![
                "alpha_link".to_string(),
                "middle_link".to_string(),
                "zeta_link".to_string()
            ]
        );
    }

    #[test]
    fn link_kind_as_str_is_stable() {
        assert_eq!(ForgeLinkKind::Link.as_str(), "link");
    }
}
