// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Cross-doc registry — build-time index of named SCXML/forge artifacts
// participating in cross-document reference resolution. Consulted by
// SCXML validators that point at names declared in sibling docs (e.g.
// `<sce:on-sample link="X">` targeting a forge link doc, or
// `<sce:outbox ref="owner.inbox">` targeting a peer statechart /
// worker doc).
//
// Background. SCE separates each document kind into its own author-
// facing schema (forge link / buffer-pool / worker; SCXML statechart),
// but consumers need to validate cross-document references at build
// time. Before this module's predecessor (`ForgeLinkRegistry`,
// shipped with the §5.E sample-callback work), no cross-schema
// reference index existed —
// SCXML validators could only consult one parsed doc at a time.
//
// Spec anchors. Watching-zenoh RFC §5.E sample-callback work
// introduced the `<sce:on-sample link="X">` cross-reference
// (link-kind axis). The RFC §5.D worker outbox surface extends the
// same registry to cover statechart + worker recipient kinds for
// `<sce:outbox ref>` resolution — single registry, cross-kind
// queries from one structure.
//
// Production wiring. The `compile_scxml_with_imports` orchestrator
// walks every input doc, populates this registry, then invokes the
// cross-ref validators — including `validate_on_sample_link_references`,
// which earlier existed without a production caller (a
// silently-broken-hook instance, now closed).

use std::collections::HashMap;

use super::model::ForgeDocument;

/// SCXML document kinds that participate in the cross-document
/// reference surface. Adding a kind here forces exhaustive matches at
/// every consumer (e.g. `<sce:on-sample>` validator's wrong-kind
/// diagnostic), which is precisely the drift protection we want for
/// cross-schema names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScxmlDocKind {
    /// `<scxml sce:kind="link">` — byte-stream link endpoint
    /// (§5.C). Today the only kind a SCXML
    /// `<sce:on-sample link="X">` reference may resolve to.
    Link,
    /// `<scxml>` — W3C SCXML statechart (no `sce:kind` attribute, or
    /// `sce:kind="statechart"` explicit). Outbox refs (`<sce:outbox
    /// ref="owner.inbox">`) may resolve to this kind per RFC §5.D
    /// line 895 example.
    Statechart,
    /// `<scxml sce:kind="worker">` — C2 worker doc. Outbox refs may
    /// resolve to this kind too per RFC §5.D line 911 ("any non-inbox
    /// access" by negation admits inbox access regardless of owner
    /// kind).
    Worker,
}

impl ScxmlDocKind {
    /// Slash-path label for this kind — used in diagnostic messages
    /// rather than `Debug` so the wire form stays stable across Rust
    /// edition / `Debug`-impl changes.
    pub fn as_str(&self) -> &'static str {
        match self {
            ScxmlDocKind::Link => "link",
            ScxmlDocKind::Statechart => "statechart",
            ScxmlDocKind::Worker => "worker",
        }
    }
}

/// Build-time cross-document name index. Populated once per build by
/// walking every parsed forge document (via [`Self::record_document`])
/// and every parsed SCXML statechart (via [`Self::record_statechart`]);
/// consulted by SCXML validators that need to resolve a name reference
/// (e.g. `<sce:on-sample link="X">`, `<sce:outbox ref="owner.inbox">`).
///
/// Names are unique across all doc kinds — duplicates are rejected at
/// registration time so `lookup` can return a single answer.
/// Forge itself already enforces unique artifact names within its own
/// build, and SCXML statechart names enter through a separate channel,
/// so a duplicate surfacing here usually indicates either a producer
/// bug or a name collision between an SCXML statechart and a forge
/// doc — both worth surfacing as a build-time error.
///
/// `stage_pools` is a sparse parallel map keyed by link names —
/// populated only for links that declare `<sce:stage-pool ref="X"/>`
/// (RFC §5.E sample-callback surface). The keys are a strict subset of the
/// link entries in `docs`. Consumers query it via
/// [`Self::lookup_stage_pool`] to wire the SCXML on-sample
/// validator's `pool/sample-take-without-stage-pool` diagnostic.
#[derive(Debug, Default)]
pub struct SceCrossDocRegistry {
    docs: HashMap<String, ScxmlDocKind>,
    /// Sparse: only links with a declared `<sce:stage-pool>` element
    /// have an entry. Absence == None == link's `Sample::take()`
    /// resolves to the runtime's `PanicOnTakeHook` default.
    stage_pools: HashMap<String, String>,
}

impl SceCrossDocRegistry {
    /// Empty registry. Caller registers entries by walking the build's
    /// parsed documents.
    pub fn new() -> Self {
        Self {
            docs: HashMap::new(),
            stage_pools: HashMap::new(),
        }
    }

    /// Register one artifact without per-doc metadata. Returns `Err`
    /// with the existing kind if the name is already registered with a
    /// different kind; no-op if the same name + same kind is re-
    /// registered (idempotent for repeat calls during incremental
    /// builds). Use [`Self::record_document`] for production code on
    /// forge docs and [`Self::record_statechart`] for SCXML docs —
    /// this entry point is convenience for tests that only exercise
    /// the kind-resolution surface.
    pub fn record(
        &mut self,
        name: impl Into<String>,
        kind: ScxmlDocKind,
    ) -> Result<(), ScxmlDocKind> {
        let name = name.into();
        match self.docs.get(&name) {
            Some(existing) if *existing == kind => Ok(()),
            Some(existing) => Err(*existing),
            None => {
                self.docs.insert(name, kind);
                Ok(())
            }
        }
    }

    /// Register a forge document by its declared kind. Today's
    /// supported kinds: link (records as [`ScxmlDocKind::Link`] +
    /// optionally captures `<sce:stage-pool ref>` from `LinkModel`),
    /// worker (records as [`ScxmlDocKind::Worker`]). Other forge
    /// kinds (algorithm, codec, buffer-pool, timer, …) are no-op
    /// because they are not valid outbox / on-sample recipients.
    pub fn record_document(&mut self, doc: &ForgeDocument) -> Result<(), ScxmlDocKind> {
        match doc {
            ForgeDocument::Link(link) => {
                self.record(link.name.clone(), ScxmlDocKind::Link)?;
                if let Some(stage_pool) = link.stage_pool.as_ref() {
                    // First registration wins — duplicate links are
                    // already rejected by `record` above, so we never
                    // reach this branch with conflicting stage_pool refs.
                    self.stage_pools
                        .entry(link.name.clone())
                        .or_insert_with(|| stage_pool.clone());
                }
                Ok(())
            }
            ForgeDocument::Worker(worker) => self.record(worker.name.clone(), ScxmlDocKind::Worker),
            _ => Ok(()),
        }
    }

    /// Register an SCXML statechart document by its `<scxml name>`
    /// attribute. SCXML statecharts don't flow through `ForgeDocument`
    /// (separate parse pipeline), so they enter the registry through
    /// this dedicated entry point. Returns `Err` with the existing
    /// kind if the name collides with a previously-registered doc
    /// (link / worker / another statechart).
    pub fn record_statechart(&mut self, name: impl Into<String>) -> Result<(), ScxmlDocKind> {
        self.record(name, ScxmlDocKind::Statechart)
    }

    /// Resolve a name reference. `None` means the name is not declared
    /// in any input doc. Consumers raise the appropriate cross-ref
    /// diagnostic (e.g. `scxml/on-sample-link-not-declared`,
    /// `worker/outbox-ref-unknown`).
    pub fn lookup(&self, name: &str) -> Option<ScxmlDocKind> {
        self.docs.get(name).copied()
    }

    /// Resolve the stage-copy pool reference for a registered link.
    /// `None` means the link has no `<sce:stage-pool>` element OR the
    /// link itself is not registered. Callers must verify link
    /// registration via [`Self::lookup`] separately when
    /// distinguishing "unregistered link" from "registered link
    /// without stage pool" (the latter raises
    /// `pool/sample-take-without-stage-pool`).
    pub fn lookup_stage_pool(&self, name: &str) -> Option<&str> {
        self.stage_pools.get(name).map(String::as_str)
    }

    /// Sorted list of registered doc names of a given kind.
    /// Diagnostics use this for `Fix::ReplaceOneOf` candidate lists so
    /// authors see legal alternatives when their reference does not
    /// resolve.
    pub fn names_of_kind(&self, kind: ScxmlDocKind) -> Vec<String> {
        let mut out: Vec<String> = self
            .docs
            .iter()
            .filter(|(_, k)| **k == kind)
            .map(|(n, _)| n.clone())
            .collect();
        out.sort();
        out
    }

    /// Sorted union of doc names across one or more kinds. Used by
    /// outbox cross-resolution where multiple recipient kinds
    /// (statechart + worker per Q-Outbox-3 (b)) are valid candidates.
    /// Empty kinds slice yields empty Vec.
    pub fn names_of_any_kind(&self, kinds: &[ScxmlDocKind]) -> Vec<String> {
        let mut out: Vec<String> = self
            .docs
            .iter()
            .filter(|(_, k)| kinds.iter().any(|wanted| wanted == *k))
            .map(|(n, _)| n.clone())
            .collect();
        out.sort();
        out
    }

    /// Total count across all kinds. Surfaced for debugging and for
    /// tests that assert registry construction observed the expected
    /// number of artifacts.
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup_link() {
        let mut reg = SceCrossDocRegistry::new();
        assert!(reg.is_empty());
        reg.record("scout_link", ScxmlDocKind::Link).unwrap();
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.lookup("scout_link"), Some(ScxmlDocKind::Link));
        assert_eq!(reg.lookup("missing"), None);
    }

    #[test]
    fn record_idempotent_on_same_kind() {
        let mut reg = SceCrossDocRegistry::new();
        reg.record("scout_link", ScxmlDocKind::Link).unwrap();
        // Second call with the same kind succeeds — incremental
        // builds re-walking parsed documents must not raise.
        reg.record("scout_link", ScxmlDocKind::Link).unwrap();
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn record_rejects_kind_collision() {
        let mut reg = SceCrossDocRegistry::new();
        reg.record("name_x", ScxmlDocKind::Link).unwrap();
        // Same name, different kind → rejection with the existing
        // kind label so the caller can surface a meaningful conflict
        // diagnostic.
        let err = reg
            .record("name_x", ScxmlDocKind::Statechart)
            .expect_err("kind collision must reject");
        assert_eq!(err, ScxmlDocKind::Link);
    }

    #[test]
    fn record_statechart_separates_from_record_document() {
        let mut reg = SceCrossDocRegistry::new();
        reg.record_statechart("session_fsm").unwrap();
        assert_eq!(reg.lookup("session_fsm"), Some(ScxmlDocKind::Statechart));
    }

    #[test]
    fn names_of_kind_returns_sorted() {
        let mut reg = SceCrossDocRegistry::new();
        reg.record("zeta_link", ScxmlDocKind::Link).unwrap();
        reg.record("alpha_link", ScxmlDocKind::Link).unwrap();
        reg.record("middle_link", ScxmlDocKind::Link).unwrap();
        assert_eq!(
            reg.names_of_kind(ScxmlDocKind::Link),
            vec![
                "alpha_link".to_string(),
                "middle_link".to_string(),
                "zeta_link".to_string()
            ]
        );
    }

    #[test]
    fn names_of_any_kind_union_sorted_across_kinds() {
        // Q-Outbox-3 (b) outbox recipient candidates span statechart
        // + worker; this helper unifies the two name lists so the
        // outbox validator's `Fix::ReplaceOneOf` carries every legal
        // recipient regardless of kind.
        let mut reg = SceCrossDocRegistry::new();
        reg.record_statechart("session_fsm").unwrap();
        reg.record("tx_loop", ScxmlDocKind::Worker).unwrap();
        reg.record("rx_loop", ScxmlDocKind::Worker).unwrap();
        reg.record("link_a", ScxmlDocKind::Link).unwrap();
        let candidates = reg.names_of_any_kind(&[ScxmlDocKind::Statechart, ScxmlDocKind::Worker]);
        assert_eq!(
            candidates,
            vec![
                "rx_loop".to_string(),
                "session_fsm".to_string(),
                "tx_loop".to_string(),
            ]
        );
    }

    #[test]
    fn record_via_record_only_has_no_stage_pool() {
        // The convenience `record` API (used by tests of the kind-only
        // resolution surface) leaves `stage_pools` empty — only
        // `record_document` extracts it from a parsed `LinkModel`.
        let mut reg = SceCrossDocRegistry::new();
        reg.record("scout_link", ScxmlDocKind::Link).unwrap();
        assert_eq!(reg.lookup_stage_pool("scout_link"), None);
    }

    #[test]
    fn record_document_captures_stage_pool() {
        use super::super::model::{BackpressurePolicy, ForgeDocument, LinkClass, LinkModel};
        let mut reg = SceCrossDocRegistry::new();
        let doc = ForgeDocument::Link(LinkModel {
            name: "scout_link".to_string(),
            class: LinkClass::Udp,
            framer: "scout_frame_codec".to_string(),
            backpressure: BackpressurePolicy::Drop,
            inbound: vec![],
            outbound: vec![],
            rx_pool: None,
            tx_pool: None,
            stage_pool: Some("scout_stage_pool".to_string()),
            accept_stage_copy_rate: false,
            source_location: None,
        });
        reg.record_document(&doc).unwrap();
        assert_eq!(reg.lookup("scout_link"), Some(ScxmlDocKind::Link));
        assert_eq!(
            reg.lookup_stage_pool("scout_link"),
            Some("scout_stage_pool")
        );
    }

    #[test]
    fn record_document_without_stage_pool_leaves_lookup_none() {
        // A link kind without `<sce:stage-pool>` registers normally,
        // but `lookup_stage_pool` returns None — that's the trigger
        // for the η' `pool/sample-take-without-stage-pool` diagnostic.
        use super::super::model::{BackpressurePolicy, ForgeDocument, LinkClass, LinkModel};
        let mut reg = SceCrossDocRegistry::new();
        let doc = ForgeDocument::Link(LinkModel {
            name: "borrow_only_link".to_string(),
            class: LinkClass::Udp,
            framer: "scout_frame_codec".to_string(),
            backpressure: BackpressurePolicy::Drop,
            inbound: vec![],
            outbound: vec![],
            rx_pool: None,
            tx_pool: None,
            stage_pool: None,
            accept_stage_copy_rate: false,
            source_location: None,
        });
        reg.record_document(&doc).unwrap();
        assert_eq!(reg.lookup("borrow_only_link"), Some(ScxmlDocKind::Link));
        assert_eq!(reg.lookup_stage_pool("borrow_only_link"), None);
    }

    #[test]
    fn record_document_worker_registers_kind() {
        // C2-α `WorkerModel` lacks any cross-doc reference fields
        // itself; the registry just records its name so outbox refs
        // pointing AT this worker can resolve.
        use super::super::model::{ForgeDocument, InboxConfig, InboxOrdering, WorkerModel};
        let mut reg = SceCrossDocRegistry::new();
        let doc = ForgeDocument::Worker(WorkerModel {
            name: "rx_loop".to_string(),
            link_rx: "udp_scout".to_string(),
            inbox: InboxConfig {
                depth: 16,
                ordering: InboxOrdering::AcqRel,
            },
            outbox: None,
            source_location: None,
        });
        reg.record_document(&doc).unwrap();
        assert_eq!(reg.lookup("rx_loop"), Some(ScxmlDocKind::Worker));
        // Workers don't have a stage_pool; lookup stays None.
        assert_eq!(reg.lookup_stage_pool("rx_loop"), None);
    }

    #[test]
    fn doc_kind_as_str_is_stable() {
        assert_eq!(ScxmlDocKind::Link.as_str(), "link");
        assert_eq!(ScxmlDocKind::Statechart.as_str(), "statechart");
        assert_eq!(ScxmlDocKind::Worker.as_str(), "worker");
    }
}
