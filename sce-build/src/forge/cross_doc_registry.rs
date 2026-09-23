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
// shipped with the §synth-5-E sample-callback work), no cross-schema
// reference index existed —
// SCXML validators could only consult one parsed doc at a time.
//
// Spec anchors. SCE Protocol-Synthesis RFC §synth-5-E sample-callback work
// introduced the `<sce:on-sample link="X">` cross-reference
// (link-kind axis). The RFC §synth-5-D worker outbox surface extends the
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
    /// (§synth-5-C). Today the only kind a SCXML
    /// `<sce:on-sample link="X">` reference may resolve to.
    Link,
    /// `<scxml>` — W3C SCXML statechart (no `sce:kind` attribute, or
    /// `sce:kind="statechart"` explicit). Outbox refs (`<sce:outbox
    /// ref="owner.inbox">`) may resolve to this kind per RFC §synth-5-D
    /// line 895 example.
    Statechart,
    /// `<scxml sce:kind="worker">` — C2 worker doc. Outbox refs may
    /// resolve to this kind too per RFC §synth-5-D line 911 ("any non-inbox
    /// access" by negation admits inbox access regardless of owner
    /// kind).
    Worker,
    /// `<scxml sce:kind="codec">` — wire codec doc (§synth-5-C). The
    /// kind a link's `<sce:framer ref>` resolves to; recorded so the
    /// cross-doc framer join can tell an undeclared name from a name
    /// declared under a different kind, the same distinction the
    /// on-sample validator draws for links.
    Codec,
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
            ScxmlDocKind::Codec => "codec",
        }
    }
}

/// Build-time cross-document name index. Populated once per build by
/// walking every parsed forge document (via [`Self::record_document`])
/// and every parsed SCXML statechart (via [`Self::record_statechart`]);
/// consulted by SCXML validators that need to resolve a name reference
/// (e.g. `<sce:on-sample link="X">`, `<sce:outbox ref="owner.inbox">`).
///
/// Names are unique across all doc kinds, so `lookup` has one answer —
/// and this index is not what makes them so. The build refuses a second
/// document of any kind under a taken name before either reaches here
/// (`manifest/duplicate-document-name`, from the one place that knows
/// both documents' paths), so registration only records.
///
/// ⚠ This comment used to say forge already enforced unique names and
/// that a same-kind repeat was an idempotent no-op "for incremental
/// builds". Neither held: no caller registers twice, and two link
/// documents named alike passed straight through — the second one's
/// artifacts overwrote the first's with the run reporting success.
///
/// `stage_pools` is a sparse parallel map keyed by link names —
/// populated only for links that declare `<sce:stage-pool ref="X"/>`
/// (RFC §synth-5-E sample-callback surface). The keys are a strict subset of the
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

    /// Register one artifact without per-doc metadata. The name must not
    /// be registered yet — the caller's namespace check guarantees it
    /// (see the type's docs). Use [`Self::record_document`] for
    /// production code on forge docs and [`Self::record_statechart`] for
    /// SCXML docs — this entry point is convenience for tests that only
    /// exercise the kind-resolution surface.
    pub fn record(&mut self, name: impl Into<String>, kind: ScxmlDocKind) {
        let previous = self.docs.insert(name.into(), kind);
        debug_assert!(
            previous.is_none(),
            "a document name reached the cross-doc registry twice; the \
             build's namespace check must refuse it first"
        );
    }

    /// Register a forge document by its declared kind. Today's
    /// supported kinds: link (records as [`ScxmlDocKind::Link`] +
    /// optionally captures `<sce:stage-pool ref>` from `LinkModel`),
    /// worker (records as [`ScxmlDocKind::Worker`]), codec (records as
    /// [`ScxmlDocKind::Codec`], the target kind of a link's
    /// `<sce:framer ref>`). Other forge kinds (algorithm, buffer-pool,
    /// timer, …) are no-op because no cross-document reference names
    /// them — buffer-pool refs resolve through
    /// [`super::pool_registry::ForgePoolRegistry`], which carries the
    /// slot geometry those joins also need.
    pub fn record_document(&mut self, doc: &ForgeDocument) {
        match doc {
            ForgeDocument::Link(link) => {
                self.record(link.name.clone(), ScxmlDocKind::Link);
                if let Some(stage_pool) = link.stage_pool.as_ref() {
                    self.stage_pools
                        .insert(link.name.clone(), stage_pool.clone());
                }
            }
            ForgeDocument::Worker(worker) => self.record(worker.name.clone(), ScxmlDocKind::Worker),
            ForgeDocument::Codec(codec) => self.record(codec.name.clone(), ScxmlDocKind::Codec),
            _ => {}
        }
    }

    /// Register an SCXML statechart document by its `<scxml name>`
    /// attribute. SCXML statecharts don't flow through `ForgeDocument`
    /// (separate parse pipeline), so they enter the registry through
    /// this dedicated entry point. The name is unique by the time it
    /// arrives, as for [`Self::record`].
    pub fn record_statechart(&mut self, name: impl Into<String>) {
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
    /// (statechart + worker) are valid candidates.
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
        reg.record("scout_link", ScxmlDocKind::Link);
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.lookup("scout_link"), Some(ScxmlDocKind::Link));
        assert_eq!(reg.lookup("missing"), None);
    }

    #[test]
    fn record_statechart_separates_from_record_document() {
        let mut reg = SceCrossDocRegistry::new();
        reg.record_statechart("session_fsm");
        assert_eq!(reg.lookup("session_fsm"), Some(ScxmlDocKind::Statechart));
    }

    #[test]
    fn names_of_kind_returns_sorted() {
        let mut reg = SceCrossDocRegistry::new();
        reg.record("zeta_link", ScxmlDocKind::Link);
        reg.record("alpha_link", ScxmlDocKind::Link);
        reg.record("middle_link", ScxmlDocKind::Link);
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
        // Outbox recipient candidates span statechart
        // + worker; this helper unifies the two name lists so the
        // outbox validator's `Fix::ReplaceOneOf` carries every legal
        // recipient regardless of kind.
        let mut reg = SceCrossDocRegistry::new();
        reg.record_statechart("session_fsm");
        reg.record("tx_loop", ScxmlDocKind::Worker);
        reg.record("rx_loop", ScxmlDocKind::Worker);
        reg.record("link_a", ScxmlDocKind::Link);
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
        reg.record("scout_link", ScxmlDocKind::Link);
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
        reg.record_document(&doc);
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
        // for the sample-callback `pool/sample-take-without-stage-pool` diagnostic.
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
        reg.record_document(&doc);
        assert_eq!(reg.lookup("borrow_only_link"), Some(ScxmlDocKind::Link));
        assert_eq!(reg.lookup_stage_pool("borrow_only_link"), None);
    }

    #[test]
    fn record_document_worker_registers_kind() {
        // Item C2 `WorkerModel` lacks any cross-doc reference fields
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
        reg.record_document(&doc);
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
