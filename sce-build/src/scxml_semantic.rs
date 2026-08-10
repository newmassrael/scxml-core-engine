//! SCXML semantic-validation errors (§wire-W5 producer side).
//!
//! §scxml-3 reference-resolution and §5.8 top-level-script
//! rejection failures detected after the document parses
//! successfully but before the model can be code-generated.
//!
//! Architectural note: these errors are deliberately separated
//! from `forge::error::ValidationError`. The forge enum is scoped
//! to forge-document structure rules (codec/transform/procedure
//! kinds, sce:context handling, etc.) per its file-level doc; SCXML
//! semantic rules come from a different specification (W3C SCXML)
//! with different repair surfaces. §wire-W5 D4 documents the
//! decision to keep these enums parallel rather than generalize
//! `ValidationError` to admit non-forge document kinds.
//!
//! Wire-code mapping (§wire-W5 D2 — §wire-W4 D4 fold precedent):
//! - [`ScxmlSemanticError::InitialStateUnknown`] →
//!   `validation/invalid-reference` (REUSE — concept identity
//!   with forge `ValidationError::InvalidReference`)
//! - [`ScxmlSemanticError::TransitionTargetUnknown`] →
//!   `validation/invalid-reference` (REUSE)
//! - [`ScxmlSemanticError::HistoryDefaultTransitionMissing`] →
//!   `validation/missing-element` (REUSE — concept identity with forge
//!   `ValidationError::MissingElement`: a required child element is
//!   absent)
//! - [`ScxmlSemanticError::NoStates`] →
//!   `validation/empty-collection` (REUSE)
//! - [`ScxmlSemanticError::TopLevelScriptUnloaded`] →
//!   `scxml/top-level-script-unloaded` (NEW — §scxml-5.8 has
//!   no forge analog)

use thiserror::Error;

/// Disambiguates the document scope for an unresolved `initial`
/// attribute. The wire code is the same
/// (`validation/invalid-reference`) for both, but consumers
/// constructing repair guidance benefit from knowing whether the
/// rename surface is the document root or a compound state's
/// children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitialStateScope {
    /// `<scxml initial="...">` references an undeclared state.
    DocumentRoot,
    /// `<state initial="..." id="<parent>">` references a state
    /// not in `<parent>`'s direct children. `parent_id` carries the
    /// owning state so repair tools can scope candidate suggestions.
    CompoundState { parent_id: String },
}

impl std::fmt::Display for InitialStateScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitialStateScope::DocumentRoot => write!(f, "document root"),
            InitialStateScope::CompoundState { parent_id } => {
                write!(f, "state '{parent_id}'")
            }
        }
    }
}

/// SCXML post-parse semantic failures.
///
/// Each variant maps to a stable wire `DiagnosticCode` via
/// [`crate::forge::diagnostic::scxml_semantic_fields`]. The mapping
/// is intentional — of the five variants that pair with a C++
/// `Semantic<Variant>` leaf, four reuse forge `validation/*` codes per
/// the §wire-W4 D4 fold precedent (concept identity over namespace
/// duplication) and one introduces `scxml/top-level-script-unloaded`.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ScxmlSemanticError {
    /// `initial` attribute names a state that is not declared.
    /// Covers both root-level (`<scxml initial="X">`) and compound
    /// state (`<state id="P" initial="X">`) cases — `scope`
    /// distinguishes. Mirrors C++
    /// `SCE::parsing::SemanticInitialStateUnknown` (§wire-W5 D1).
    #[error("Initial state '{state_id}' not found ({scope})")]
    InitialStateUnknown {
        state_id: String,
        scope: InitialStateScope,
        /// All declared state ids — used for repair-candidate
        /// suggestions. Empty when the model has no states (caller
        /// should emit [`ScxmlSemanticError::NoStates`] instead).
        available: Vec<String>,
    },

    /// `<transition target="X">` references a state that is not
    /// declared. Mirrors C++ `SemanticTransitionTargetUnknown`.
    #[error("Transition in state '{state}' references non-existent target state '{target}'")]
    TransitionTargetUnknown {
        /// Owning state id (the `<state>` containing the bad
        /// `<transition>`).
        state: String,
        /// Unresolved target id.
        target: String,
        /// All declared state ids — used for repair candidates.
        available: Vec<String>,
    },

    /// A `<history>` element declares no default configuration. The
    /// spec requires a single unconditional `<transition>` child naming
    /// the configuration to enter when the parent has no stored
    /// history; without it the pseudostate can never be entered, so the
    /// declaration is unusable rather than merely incomplete.
    ///
    /// The rule is a declaration rule, not a use rule: the child is
    /// required whether or not any transition names the pseudostate.
    ///
    /// Mirrors C++ `SemanticHistoryDefaultMissing`.
    #[error(
        "History state '{history_id}' in state '{parent_id}' declares no \
         default <transition> — W3C SCXML 3.10.2 requires one naming the \
         configuration to enter when '{parent_id}' has no stored history"
    )]
    HistoryDefaultTransitionMissing {
        /// The `<history>` element's id.
        history_id: String,
        /// The compound state containing the `<history>`. The default
        /// configuration is restricted to this state's descendants, so
        /// it scopes the legal targets.
        parent_id: String,
        /// `parent_id`'s children in document order — the legal
        /// default-configuration set the author picks from.
        available: Vec<String>,
    },

    /// SCXML document has no top-level `<state>`, `<parallel>`, or
    /// `<final>`. §scxml-3.2 requires at least one root state.
    /// Mirrors C++ `SemanticNoStates`.
    #[error("No state nodes found in SCXML document")]
    NoStates,

    /// Top-level `<script>` element either (a) has empty content
    /// AND empty `src`, or (b) has `src` but the file failed to
    /// load. §scxml-5.8 mandates document rejection in either
    /// case.
    ///
    /// Payload fields are optional because the producer site
    /// (`SCXMLParser::parse_global_scripts` setting
    /// `model.document_rejected = true`) does not capture
    /// the index/src of the failing script. C++ side captures both
    /// via `SemanticTopLevelScriptUnloaded(index, src)`. Wire-code
    /// dispatch is sufficient for test-as-consumer drift pinning;
    /// payload-detail asymmetry is acceptable per the §wire-W5
    /// anti-pattern note ("NEW wire code count > NEW Rust producer
    /// count" — both sides emit the same wire code).
    ///
    /// Mirrors C++ `SemanticTopLevelScriptUnloaded`.
    #[error("Top-level <script> rejected per W3C SCXML 5.8")]
    TopLevelScriptUnloaded {
        /// 1-based script element index (None when producer site
        /// doesn't capture it; analyzer.rs path always emits None).
        index: Option<usize>,
        /// `src` attribute value when known (None for empty-script
        /// rejection or analyzer.rs path).
        src: Option<String>,
    },

    /// A `<state>` / `<parallel>` / `<final>` declared in the
    /// document graph cannot be entered from any execution path that
    /// the parser-stage BFS can derive. The walk seeds at the
    /// document `initial` (or default-first-child when omitted),
    /// follows compound-state initial cascade (§scxml-3.3),
    /// enters every non-history child of a `<parallel>` (W3C SCXML
    /// §3.4), and follows every transition `target` edge plus every
    /// history pseudostate `default_target` (§scxml-3.10). A
    /// state outside the closure is dead code — keeping it through
    /// codegen wastes generated-state surface and masks authoring
    /// mistakes that produce orphan subgraphs (a recurring
    /// generated-SCXML failure mode).
    ///
    /// Emitted in preference to `DeadTransition` only when the
    /// orphan state has zero `<transition>` children — when at least
    /// one transition is present, the per-transition variant fires
    /// first because it points the author at a concrete element to
    /// repair (delete the transition / re-attach the source state).
    #[error("State '{state_id}' is unreachable from the document initial configuration")]
    UnreachableState {
        /// The unreachable state's id.
        state_id: String,
    },

    /// A `<transition>` element lives in a state that is itself
    /// unreachable per the BFS described on [`Self::UnreachableState`].
    /// The transition's `target` may name a perfectly valid state
    /// (and that state may even be reachable through a different
    /// path); the transition still cannot fire because its source
    /// state is never entered.
    ///
    /// Carried in preference to the bare-`UnreachableState` form
    /// when an unreachable state contains transitions — `source` +
    /// `target` together name the specific orphan edge so author
    /// repair lands at one element instead of inferring it from the
    /// state-level diagnostic.
    #[error(
        "Transition in unreachable state '{state}' targets '{target}' — \
         source state is never entered"
    )]
    DeadTransition {
        /// Owning state id (the unreachable `<state>` / `<parallel>`
        /// containing the `<transition>`). Named `state` rather than
        /// `source` so it does not collide with thiserror's reserved
        /// `source` field (which routes to the error-chain accessor).
        state: String,
        /// The transition `target` attribute verbatim (multi-target
        /// space-separated declarations carry the full string —
        /// every target shares the same orphan source so collapsing
        /// to a single diagnostic is correct).
        target: String,
    },

    /// A compound `<state>` has sibling children that disagree on
    /// whether a given event is handled, with no parent-level
    /// fallthrough to absorb the gap. AI-generated SCXML produces
    /// this constantly — the model emits handler sets for some
    /// siblings and forgets the others — but the parser cannot tell
    /// whether the gap is the author's intent or a typo.
    ///
    /// The validator is intentionally narrow to keep false positives
    /// at zero across the W3C IRP, conformance, and downstream
    /// consumer corpora: it fires only when the sibling children form a
    /// shared event vocabulary (there exists at least one event
    /// matched by every transition-carrying sibling — the "common
    /// ground" precondition) AND a specific event is matched by some
    /// siblings but not others AND the parent itself has no
    /// fallthrough. This excludes the test207-style "sequential
    /// protocol stages with disjoint event vocabularies" pattern
    /// that prevails in the W3C IRP suite.
    ///
    /// Author escape hatch when the intent gap is genuine:
    /// `sce:exhaustive="false"` on the compound parent silences this
    /// diagnostic for that parent only.
    #[error(
        "Compound state '{parent}' has children handling event \
         '{event}' inconsistently — handlers: {handlers:?}, \
         non-handlers: {non_handlers:?}. Add the missing transition, \
         add a parent-level fallthrough, or annotate the parent with \
         sce:exhaustive=\"false\" if the gap is intentional."
    )]
    NonExhaustiveEventHandling {
        /// Compound state id whose children disagree on event
        /// coverage.
        parent: String,
        /// Event name (literal token form, no wildcard) that some
        /// siblings handle and others do not.
        event: String,
        /// Sibling ids that match `event` via direct/prefix/wildcard
        /// matching, in document order.
        handlers: Vec<String>,
        /// Sibling ids that do not match `event` and lack a
        /// catch-all, in document order. Author repair lands on one
        /// of these (add the missing transition, or annotate the
        /// parent as deliberately non-exhaustive).
        non_handlers: Vec<String>,
    },

    /// A `<transition cond="...">` carries a guard expression whose
    /// value is statically determinable as `false`, so the
    /// transition can never fire. The validator stops short of full
    /// SMT — only trivial cases are recognised:
    ///
    ///   * The literal `false` (ECMAScript convention; lowercase
    ///     per W3C SCXML §B).
    ///   * The numeric literal `0`.
    ///   * A binary equality `N == M` where both sides parse as
    ///     numeric literals with differing values (`1==2`, `0==1`).
    ///   * A binary inequality `N != M` where both sides parse as
    ///     numeric literals with equal values (`1!=1`, `0!=0`).
    ///
    /// Language-prefixed conditions (`cpp:`, `kotlin:`, `rust:`)
    /// remain opaque to the validator — their semantics depend on
    /// the host language's expression evaluator, which the parser
    /// cannot statically inspect. See `docs/SCE_ACCEPTED_SUBSET.md`
    /// for the full opacity contract.
    #[error(
        "Transition in state '{state}' carries guard '{cond}' that \
         is statically false — the transition can never fire. \
         Remove the transition or change the guard expression."
    )]
    AlwaysFalseGuard {
        /// Owning state id.
        state: String,
        /// The raw `cond` attribute text the validator classified as
        /// statically false.
        cond: String,
    },

    /// A `<transition>` is shadowed by an earlier unconditional
    /// sibling: per §scxml-5.10 transition selection, the first
    /// matching transition in document order fires, and an
    /// unconditional transition matching the same event family
    /// (cond empty / cond literal `true` / cond literal `1`) makes
    /// every later same-event transition unreachable.
    ///
    /// The validator only flags the strict prefix-superset case:
    /// the shadowing transition's event descriptor must literally
    /// equal the shadowed transition's event descriptor (including
    /// the empty / `*` cases), and the shadowing transition must
    /// carry no guard. Token-prefix superset cases (`event="foo"`
    /// shadowing `event="foo.bar"`) are deliberately not flagged —
    /// the relative priority depends on ancestor-priority rules
    /// that the parser-stage walker cannot disambiguate without
    /// running the full runtime selection algorithm.
    #[error(
        "Transition #{shadowed_index} in state '{state}' (event \
         '{event}') is shadowed by an earlier unconditional \
         transition #{shadowing_index} with the same event \
         descriptor. The shadowed transition can never fire. \
         Reorder the transitions, add a guard to the shadowing \
         transition, or remove the shadowed transition."
    )]
    ShadowedTransition {
        /// Owning state id.
        state: String,
        /// Event descriptor verbatim (the literal `event` attribute
        /// value of the shadowed transition; same as the shadowing
        /// one by construction).
        event: String,
        /// 0-based document-order index of the unconditional
        /// shadowing transition within the state's transition list.
        shadowing_index: usize,
        /// 0-based document-order index of the shadowed transition
        /// (the later one in the list).
        shadowed_index: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
    use crate::forge::error::ForgeError;

    fn single_code(err: &ForgeError) -> &'static str {
        let diags = err.to_diagnostics();
        assert_eq!(
            diags.len(),
            1,
            "expected single diagnostic, got {}",
            diags.len()
        );
        diags[0].code.as_str()
    }

    /// Each variant must map to its declared wire code. The §wire-W5 D2
    /// mapping is load-bearing for the §wire-W4 D4 fold precedent — fold
    /// claims must be testable at the variant level, not just at
    /// the catalog level.
    #[test]
    fn initial_state_unknown_emits_validation_invalid_reference() {
        let err: ForgeError = ScxmlSemanticError::InitialStateUnknown {
            state_id: "armed".into(),
            scope: InitialStateScope::DocumentRoot,
            available: vec!["idle".into(), "running".into()],
        }
        .into();
        assert_eq!(single_code(&err), "validation/invalid-reference");
    }

    #[test]
    fn initial_state_unknown_compound_scope_emits_same_code() {
        // The compound-state vs document-root distinction lives in
        // payload, not in the wire code — both surfaces map to the
        // same `validation/invalid-reference` per §wire-W5 D2 (one
        // C++ leaf `SemanticInitialStateUnknown` covers both).
        let err: ForgeError = ScxmlSemanticError::InitialStateUnknown {
            state_id: "deep".into(),
            scope: InitialStateScope::CompoundState {
                parent_id: "outer".into(),
            },
            available: vec!["a".into(), "b".into()],
        }
        .into();
        assert_eq!(single_code(&err), "validation/invalid-reference");
    }

    #[test]
    fn transition_target_unknown_emits_validation_invalid_reference() {
        let err: ForgeError = ScxmlSemanticError::TransitionTargetUnknown {
            state: "active".into(),
            target: "ghost".into(),
            available: vec!["idle".into()],
        }
        .into();
        assert_eq!(single_code(&err), "validation/invalid-reference");
    }

    #[test]
    fn history_default_transition_missing_emits_validation_missing_element() {
        let err: ForgeError = ScxmlSemanticError::HistoryDefaultTransitionMissing {
            history_id: "resume".into(),
            parent_id: "session".into(),
            available: vec!["opening".into(), "running".into()],
        }
        .into();
        assert_eq!(single_code(&err), "validation/missing-element");
    }

    /// `validation/missing-element` carries `actual` and no `fix`:
    /// SCE_ERROR_CONTRACT §3.1 declares no add-child-element fix
    /// variant, and the catalog row (§5.1) records the code as
    /// fix-free. The C++ counterpart `SemanticHistoryDefaultMissing`
    /// makes the same choice — a `fix` on one side only would make two
    /// producers disagree about a shared wire code.
    #[test]
    fn history_default_transition_missing_names_the_element_without_a_fix() {
        use crate::forge::diagnostic::ToDiagnostics;

        let err: ForgeError = ScxmlSemanticError::HistoryDefaultTransitionMissing {
            history_id: "resume".into(),
            parent_id: "session".into(),
            available: vec!["opening".into()],
        }
        .into();
        let diags = err.to_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].actual.as_deref(), Some("resume"));
        assert!(
            diags[0].fix.is_none(),
            "missing-element has no structured repair on the wire, got {:?}",
            diags[0].fix
        );
    }

    #[test]
    fn no_states_emits_validation_empty_collection() {
        let err: ForgeError = ScxmlSemanticError::NoStates.into();
        assert_eq!(single_code(&err), "validation/empty-collection");
    }

    #[test]
    fn top_level_script_unloaded_emits_scxml_top_level_script_unloaded() {
        // The 1 NEW wire code §wire-W5 D2 introduces. The Rust
        // analyzer path emits with both fields None; the C++ parser
        // side carries index/src detail.
        let err: ForgeError = ScxmlSemanticError::TopLevelScriptUnloaded {
            index: None,
            src: None,
        }
        .into();
        assert_eq!(single_code(&err), "scxml/top-level-script-unloaded");
    }

    #[test]
    fn top_level_script_unloaded_with_detail_keeps_same_code() {
        // Payload variation MUST NOT change wire code — the wire
        // code is keyed by error concept, not payload shape. C++
        // side emits with detail; Rust analyzer path emits without;
        // the drift test (`cpp_scxml_semantic_*`) pins both sides
        // agree on the code, not on the payload shape (§wire-W5
        // anti-pattern note).
        let err: ForgeError = ScxmlSemanticError::TopLevelScriptUnloaded {
            index: Some(2),
            src: Some("scripts/init.js".into()),
        }
        .into();
        assert_eq!(single_code(&err), "scxml/top-level-script-unloaded");
    }

    /// Drift guard: every `ScxmlSemanticError` variant must be
    /// reachable from `ForgeError::Scxml` via `From`. Without this,
    /// adding a variant could leave an orphan path that the
    /// `forge_error_fields` exhaustive match never sees.
    #[test]
    fn every_variant_routes_through_forge_error() {
        let variants: Vec<ScxmlSemanticError> = vec![
            ScxmlSemanticError::InitialStateUnknown {
                state_id: "x".into(),
                scope: InitialStateScope::DocumentRoot,
                available: vec![],
            },
            ScxmlSemanticError::TransitionTargetUnknown {
                state: "s".into(),
                target: "t".into(),
                available: vec![],
            },
            ScxmlSemanticError::HistoryDefaultTransitionMissing {
                history_id: "h".into(),
                parent_id: "p".into(),
                available: vec![],
            },
            ScxmlSemanticError::NoStates,
            ScxmlSemanticError::TopLevelScriptUnloaded {
                index: None,
                src: None,
            },
            ScxmlSemanticError::UnreachableState {
                state_id: "ghost".into(),
            },
            ScxmlSemanticError::DeadTransition {
                state: "ghost".into(),
                target: "armed".into(),
            },
            ScxmlSemanticError::NonExhaustiveEventHandling {
                parent: "dispatch".into(),
                event: "cmd.stop".into(),
                handlers: vec!["idle".into(), "stopped".into()],
                non_handlers: vec!["active".into()],
            },
            ScxmlSemanticError::AlwaysFalseGuard {
                state: "armed".into(),
                cond: "false".into(),
            },
            ScxmlSemanticError::ShadowedTransition {
                state: "armed".into(),
                event: "fire".into(),
                shadowing_index: 0,
                shadowed_index: 1,
            },
        ];
        // The list above is not self-checking: a new variant reaches
        // `ForgeError` through the blanket `From` impl, so omitting it
        // here left this test green while claiming to cover "every
        // variant". `variant_name`'s match is exhaustive, so a new
        // variant reds the build until it is named there — and the
        // distinct-name count below then reds until it is also added
        // to the list. Both directions are load-bearing.
        let named: std::collections::BTreeSet<&'static str> =
            variants.iter().map(variant_name).collect();
        assert_eq!(
            named.len(),
            VARIANT_COUNT,
            "every ScxmlSemanticError variant must appear in the list \
             above — missing: the arms of `variant_name` not present in \
             {named:?}"
        );

        for v in variants {
            let err: ForgeError = v.into();
            // Constructing the conversion and rendering the wire
            // payload is the assertion — a variant with no
            // `scxml_semantic_fields` arm panics here.
            let _ = err.to_diagnostics();
        }
    }

    /// Number of arms in [`variant_name`]. Kept next to it so the two
    /// move together.
    const VARIANT_COUNT: usize = 10;

    /// Exhaustive discriminant projection — the compile-time half of
    /// `every_variant_routes_through_forge_error`'s coverage claim.
    fn variant_name(v: &ScxmlSemanticError) -> &'static str {
        match v {
            ScxmlSemanticError::InitialStateUnknown { .. } => "InitialStateUnknown",
            ScxmlSemanticError::TransitionTargetUnknown { .. } => "TransitionTargetUnknown",
            ScxmlSemanticError::HistoryDefaultTransitionMissing { .. } => {
                "HistoryDefaultTransitionMissing"
            }
            ScxmlSemanticError::NoStates => "NoStates",
            ScxmlSemanticError::TopLevelScriptUnloaded { .. } => "TopLevelScriptUnloaded",
            ScxmlSemanticError::UnreachableState { .. } => "UnreachableState",
            ScxmlSemanticError::DeadTransition { .. } => "DeadTransition",
            ScxmlSemanticError::NonExhaustiveEventHandling { .. } => "NonExhaustiveEventHandling",
            ScxmlSemanticError::AlwaysFalseGuard { .. } => "AlwaysFalseGuard",
            ScxmlSemanticError::ShadowedTransition { .. } => "ShadowedTransition",
        }
    }

    /// W4 D4 fold success criterion (applied symmetrically to W5):
    /// SCXML semantic failures and forge validation failures must
    /// share the SAME wire code when the concept is identical.
    /// A consumer dispatching on `validation/invalid-reference`
    /// receives both `ValidationError::InvalidReference` (forge)
    /// and `ScxmlSemanticError::InitialStateUnknown`/
    /// `TransitionTargetUnknown` (SCXML) — confirming the fold is
    /// honest at the wire level.
    #[test]
    fn fold_invariant_holds_for_invalid_reference() {
        use crate::forge::error::ValidationError;
        use crate::forge::model::ForgeKind;

        let scxml_err: ForgeError = ScxmlSemanticError::TransitionTargetUnknown {
            state: "active".into(),
            target: "ghost".into(),
            available: vec![],
        }
        .into();

        let forge_err: ForgeError = ValidationError::InvalidReference {
            kind: ForgeKind::Statechart,
            what: "transition target".into(),
            name: "ghost".into(),
            available: "active, idle".into(),
        }
        .into();

        assert_eq!(single_code(&scxml_err), single_code(&forge_err));
        assert_eq!(single_code(&forge_err), "validation/invalid-reference");

        // The same DiagnosticCode value (compile-time constant)
        // means consumer dispatch tables only need one entry per
        // wire code, not one per (doc-kind, code) pair.
        let _ = DiagnosticCode::ValidationInvalidReference;
    }

    /// W5 D6 cross-side parity: when a consumer dispatches on
    /// `code()`, SCXML and forge documents should produce the same
    /// branch for the same conceptual failure. This test pins that
    /// invariant by wire-code equality across enums.
    #[test]
    fn fold_invariant_holds_for_empty_collection() {
        use crate::forge::error::ValidationError;
        use crate::forge::model::ForgeKind;

        let scxml_err: ForgeError = ScxmlSemanticError::NoStates.into();

        let forge_err: ForgeError = ValidationError::EmptyCollection {
            kind: ForgeKind::Codec,
            what: "field".into(),
        }
        .into();

        assert_eq!(single_code(&scxml_err), single_code(&forge_err));
        assert_eq!(single_code(&forge_err), "validation/empty-collection");
    }

    // ── §wire-W5 SemanticError cross-side drift tests ──────────────
    //
    // Sister tests to §wire-W4's `cpp_parse_subtypes_match_rust_diagnostic_codes`
    // and `cpp_parse_subtype_code_returns_rust_wire_string` in
    // `sce-build/src/parser.rs`. §wire-W5 D2 inventory: 4 C++ leaves,
    // 3 fold onto existing `validation/*` wire codes (REUSE), 1
    // introduces `scxml/top-level-script-unloaded` (NEW). Cross-side
    // drift is caught when a commit edits one side without updating
    // the other.

    /// Pin the 4 C++ `Semantic<Variant>` leaves declared in
    /// `sce/include/parsing/SemanticError.h` against the §wire-W5 leaf
    /// inventory. Adding a new leaf on the C++ side without a
    /// corresponding Rust `ScxmlSemanticError` variant (or vice
    /// versa) reds this test.
    #[test]
    fn cpp_scxml_semantic_subtypes_match_rust_diagnostic_codes() {
        use std::collections::BTreeSet;

        // §wire-W5 D2 inventory: 1 NEW + 4 REUSED = 5 leaves total.
        // `SemanticHistoryDefaultMissing` joined the inventory with
        // the state-reference-resolution round.
        let rust_to_cpp: &[(&str, &str)] = &[
            (
                "validation/invalid-reference",
                "SemanticInitialStateUnknown",
            ),
            (
                "validation/invalid-reference",
                "SemanticTransitionTargetUnknown",
            ),
            (
                "validation/missing-element",
                "SemanticHistoryDefaultMissing",
            ),
            ("validation/empty-collection", "SemanticNoStates"),
            (
                "scxml/top-level-script-unloaded",
                "SemanticTopLevelScriptUnloaded",
            ),
        ];
        assert_eq!(
            rust_to_cpp.len(),
            5,
            "Expected 5 W5 leaves (§wire-W5 D2 inventory: 1 NEW + 4 REUSED)"
        );

        let expected_cpp: BTreeSet<&str> = rust_to_cpp.iter().map(|(_, cpp)| *cpp).collect();
        assert_eq!(
            expected_cpp.len(),
            5,
            "Expected 5 distinct SemanticError subtypes"
        );

        let hdr = include_str!("../../sce/include/parsing/SemanticError.h");
        let re =
            regex::Regex::new(r"class\s+(Semantic\w+)\s*:\s*public\s+SemanticError\b").unwrap();
        let mut found: BTreeSet<String> = BTreeSet::new();
        for captures in re.captures_iter(hdr) {
            found.insert(captures[1].to_string());
        }

        assert!(
            !found.is_empty(),
            "sce/include/parsing/SemanticError.h must declare at least \
             one `class Semantic<Variant> : public SemanticError` — if \
             the declaration shape changed, update this drift test in \
             the same commit"
        );

        let found_refs: BTreeSet<&str> = found.iter().map(|s| s.as_str()).collect();
        assert_eq!(
            found_refs, expected_cpp,
            "SemanticError subtype drift: C++ header = {:?}, expected \
             (§wire-W5 D2 inventory) = {:?}. Change both sides in the \
             same commit (§wire-W5).",
            found_refs, expected_cpp
        );

        // Cross-check: the NEW §wire-W5 wire code is spelled as the
        // `serde(rename = "...")` literal in
        // `sce-build/src/forge/diagnostic.rs`. Catches a future
        // rename of the wire string on the Rust side without a C++
        // counter-edit.
        let diag = include_str!("forge/diagnostic.rs");
        let needle = "\"scxml/top-level-script-unloaded\"";
        assert!(
            diag.contains(needle),
            "DiagnosticCode `scxml/top-level-script-unloaded` (paired \
             with C++ `SemanticTopLevelScriptUnloaded`) is not declared \
             as a `serde(rename)` literal in \
             sce-build/src/forge/diagnostic.rs. Keep the wire name, the \
             Rust variant, and the C++ subtype in sync — see §wire-W5."
        );
    }

    /// Pin the wire-string return literal inside each C++
    /// `Semantic<Variant>` subtype's `code()` body. The 1 NEW leaf
    /// returns `scxml/top-level-script-unloaded`; the 3 reused-code
    /// leaves return their respective folded `validation/*` strings.
    /// A rename on either side without a matching counter-edit reds
    /// here with a pointed `does not contain` diff.
    #[test]
    fn cpp_scxml_semantic_subtype_code_returns_rust_wire_string() {
        let class_to_code: &[(&str, &str)] = &[
            (
                "SemanticInitialStateUnknown",
                "validation/invalid-reference",
            ),
            (
                "SemanticTransitionTargetUnknown",
                "validation/invalid-reference",
            ),
            (
                "SemanticHistoryDefaultMissing",
                "validation/missing-element",
            ),
            ("SemanticNoStates", "validation/empty-collection"),
            (
                "SemanticTopLevelScriptUnloaded",
                "scxml/top-level-script-unloaded",
            ),
        ];
        assert_eq!(class_to_code.len(), 5);

        let hdr = include_str!("../../sce/include/parsing/SemanticError.h");

        for (cpp_class, expected_code) in class_to_code {
            let class_marker = format!("class {} : public SemanticError", cpp_class);
            let class_start = hdr.find(&class_marker).unwrap_or_else(|| {
                panic!(
                    "class `{}` not found in \
                     sce/include/parsing/SemanticError.h — drift in \
                     subtype naming, see \
                     `cpp_scxml_semantic_subtypes_match_rust_diagnostic_codes`",
                    cpp_class
                )
            });
            // Body bounds: from this class's `{` to the start of the
            // next `class ... : public SemanticError` declaration (or
            // end of header). Some leaves contain nested types
            // (`enum class Scope`) whose `};` would confuse a naive
            // find-next-`};` scanner; bounding by sibling-class
            // boundary skips the nesting issue.
            let body_start = hdr[class_start..].find('{').unwrap() + class_start + 1;
            // Bound by next `class Semantic` declaration (or EOF). The
            // `\nclass Semantic` prefix avoids matching nested `enum
            // class` keywords inside this leaf's body.
            let next_class_offset = hdr[body_start..]
                .find("\nclass Semantic")
                .map_or(hdr.len(), |rel| body_start + rel);
            let body = &hdr[body_start..next_class_offset];

            let needle = format!("return \"{}\";", expected_code);
            assert!(
                body.contains(&needle),
                "Class `{}` body does not contain `{}` — the C++ \
                 subtype's `code()` override must return the expected \
                 wire string. Update both sides in the same commit \
                 (§wire-W5 D2).",
                cpp_class,
                needle
            );
        }
    }
}
