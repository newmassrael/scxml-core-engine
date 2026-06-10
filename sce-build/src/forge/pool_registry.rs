// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Forge pool registry — build-time index of named pool artifacts declared
// in `.forge` files, consulted by deploy.yaml validators that reference
// those names (e.g. `binding.stage_pool: <forge-pool-name>`).
//
// Background. SCE separates pool *templates* (forge `<scxml
// sce:kind="buffer-pool">` documents — slot count, slot size, section,
// alignment, DMA channel, cache policy) from pool *bindings* (deploy.yaml
// fields that wire a binding to a named template). Until this module
// landed, deploy.yaml had no cross-schema reference into forge — the
// rx-pool / tx-pool pairing was a forge-internal concern (§synth-5-C link kind
// fields), and deploy.yaml validators only consulted the `DeployConfig`
// itself.
//
// Spec anchor. watching-zenoh RFC §synth-5-E (Sample API contract) introduces
// the `<sce:on-sample>` callback path whose `Sample::take()` requires a
// stage-copy destination pool, declared via `binding.stage_pool` in
// deploy.yaml. The cross-reference resolution this module implements is
// the build-time half: deploy.yaml validators query `lookup(name)` to
// confirm the referenced template exists and is the right kind.

use std::collections::HashMap;

use super::model::ForgeDocument;

/// Pool kinds that participate in the deploy.yaml cross-reference
/// surface. Adding a kind here is the textbook way to extend the
/// registry's classification surface — new variants force exhaustive
/// matches at every consumer (e.g. `stage_pool` validator's
/// "wrong-kind" diagnostic), which is precisely the drift protection
/// we want for cross-schema names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgePoolKind {
    /// `<scxml sce:kind="buffer-pool">` — fixed-slot RX/TX/stage pool
    /// (§synth-5-E). Today the only pool kind a deploy.yaml
    /// `stage_pool:` reference may resolve to.
    BufferPool,
}

impl ForgePoolKind {
    /// Slash-path label for this kind — used in diagnostic messages
    /// rather than `Debug` so the wire form stays stable across Rust
    /// edition / `Debug`-impl changes.
    pub fn as_str(&self) -> &'static str {
        match self {
            ForgePoolKind::BufferPool => "buffer-pool",
        }
    }
}

/// Build-time index of named forge pool artifacts. Populated once per
/// build by walking every parsed forge document and calling
/// [`Self::record`] (or [`Self::record_document`]); consulted by
/// deploy.yaml validators that need to resolve a name reference (e.g.
/// `binding.stage_pool: <pool-name>`).
///
/// Names are unique across all pool kinds — duplicates are rejected at
/// registration time so `lookup` can return a single answer. Forge
/// itself already enforces unique artifact names within a build, so
/// duplicates surfacing here would indicate a producer bug; the assert
/// in `record` is a defensive belt-and-braces.
#[derive(Debug, Default)]
pub struct ForgePoolRegistry {
    pools: HashMap<String, ForgePoolKind>,
}

impl ForgePoolRegistry {
    /// Empty registry. Caller registers entries by walking the forge
    /// build's parsed documents.
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
        }
    }

    /// Register one pool artifact. Returns `Err` with the existing
    /// kind if the name is already registered with a different kind;
    /// no-op if the same name + same kind is re-registered (idempotent
    /// for repeat calls during incremental builds).
    pub fn record(
        &mut self,
        name: impl Into<String>,
        kind: ForgePoolKind,
    ) -> Result<(), ForgePoolKind> {
        let name = name.into();
        match self.pools.get(&name) {
            Some(existing) if *existing == kind => Ok(()),
            Some(existing) => Err(*existing),
            None => {
                self.pools.insert(name, kind);
                Ok(())
            }
        }
    }

    /// Register a forge document if its kind is a pool kind. No-op for
    /// non-pool documents — matches the "register every parsed forge
    /// document" call-site pattern without the caller having to filter.
    pub fn record_document(&mut self, doc: &ForgeDocument) -> Result<(), ForgePoolKind> {
        if let ForgeDocument::BufferPool(pool) = doc {
            self.record(pool.name.clone(), ForgePoolKind::BufferPool)
        } else {
            Ok(())
        }
    }

    /// Resolve a name reference. `None` means the name is not declared
    /// in any `.forge` file; consumers raise the
    /// `mesh/deploy-stage-pool-not-declared` diagnostic.
    pub fn lookup(&self, name: &str) -> Option<ForgePoolKind> {
        self.pools.get(name).copied()
    }

    /// Sorted list of registered pool names of a given kind.
    /// Diagnostics use this for `Fix::ReplaceOneOf` candidate lists so
    /// authors see legal alternatives when their reference does not
    /// resolve.
    pub fn names_of_kind(&self, kind: ForgePoolKind) -> Vec<String> {
        let mut out: Vec<String> = self
            .pools
            .iter()
            .filter(|(_, k)| **k == kind)
            .map(|(n, _)| n.clone())
            .collect();
        out.sort();
        out
    }

    /// Total count across all pool kinds. Surfaced for debugging and
    /// for tests that assert registry construction observed the
    /// expected number of artifacts.
    pub fn len(&self) -> usize {
        self.pools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pools.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup_buffer_pool() {
        let mut reg = ForgePoolRegistry::new();
        assert!(reg.is_empty());
        reg.record("rx_pool_sram1", ForgePoolKind::BufferPool)
            .unwrap();
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.lookup("rx_pool_sram1"), Some(ForgePoolKind::BufferPool));
        assert_eq!(reg.lookup("missing"), None);
    }

    #[test]
    fn record_idempotent_on_same_kind() {
        let mut reg = ForgePoolRegistry::new();
        reg.record("p", ForgePoolKind::BufferPool).unwrap();
        // Second call with the same kind succeeds — incremental builds
        // re-walking parsed documents must not raise.
        reg.record("p", ForgePoolKind::BufferPool).unwrap();
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn names_of_kind_returns_sorted() {
        let mut reg = ForgePoolRegistry::new();
        reg.record("zeta_pool", ForgePoolKind::BufferPool).unwrap();
        reg.record("alpha_pool", ForgePoolKind::BufferPool).unwrap();
        reg.record("middle_pool", ForgePoolKind::BufferPool)
            .unwrap();
        assert_eq!(
            reg.names_of_kind(ForgePoolKind::BufferPool),
            vec![
                "alpha_pool".to_string(),
                "middle_pool".to_string(),
                "zeta_pool".to_string()
            ]
        );
    }

    #[test]
    fn pool_kind_as_str_is_stable() {
        assert_eq!(ForgePoolKind::BufferPool.as_str(), "buffer-pool");
    }

    #[test]
    fn record_document_buffer_pool() {
        // The convenience entry point that callers use when walking
        // every parsed `.forge` document — non-pool documents are
        // no-ops, buffer-pool documents land in the registry.
        use crate::forge::model::{BufferPoolModel, BufferPoolVariant, CachePolicy, ForgeDocument};
        let pool = BufferPoolModel {
            name: "rx_pool_sram1".to_string(),
            slot_count: 8,
            slot_size: 256,
            section: "sram1".to_string(),
            alignment: 32,
            dma_channel: None,
            cache_policy: CachePolicy::None,
            variant: BufferPoolVariant::Default,
            source_location: None,
        };
        let doc = ForgeDocument::BufferPool(pool);
        let mut reg = ForgePoolRegistry::new();
        reg.record_document(&doc).expect("buffer-pool registers");
        assert_eq!(reg.lookup("rx_pool_sram1"), Some(ForgePoolKind::BufferPool));
    }
}
