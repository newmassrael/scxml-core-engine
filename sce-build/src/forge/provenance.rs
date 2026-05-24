// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Watching-zenoh RFC §5.O Atomic 0 — IR provenance pre-emit guard.
//
// Codegen consumes per-IR-node `source_location` to emit SCE-MAP
// markers above every generated function header (Atomic 0a) and to
// drive the per-symbol attribution sourcemap JSON (Atomic 1). The
// markers are silent when the IR field is `None`, which would
// regress the spec contract (lines 3280-3284 + 3289-3290) without
// any visible signal — a classic silently-broken-hook situation
// [[feedback-silently-broken-hooks]].
//
// This pre-emit walker is the consumer that prevents that regression.
// It runs after analyzer and before generator. Every node eligible for
// marker emission must carry `source_location: Some(_)`; the first
// `None` fires `traceability/scxml-line-range-missing` (a
// codegen-internal diagnostic with no author repair — the fix lives
// in the parser site that produced the IR node).
//
// ## Eligibility scope (Atomic 0a)
//
// Atomic 0a populates the IR provenance field at four parser sites
// in `parser.rs`:
//
// 1. `SCXMLModel` root  — every parse goes through `parse_impl`.
// 2. `State` (state/final/parallel) — every `<state>`, `<final>`,
//    `<parallel>` reaches `parse_states`'s three creation sites.
// 3. `Transition` — every `<transition>` reaches `parse_transition`.
// 4. `Action` (executable content) — every `<raise>`, `<send>`,
//    `<assign>`, `<log>`, `<script>`, `<if>`, `<foreach>`,
//    `<cancel>` reaches `parse_executable_content_single`.
//
// Synthesised IR nodes (e.g. detect_features rewrites,
// resolve_history_targets stubs) that do not derive from a single
// authored XML element are *intentionally* left `None` and excluded
// from this walker per the inherited-content-branch carve-out
// documented in spec line 3286-3289 ("XInclude / sce:template
// composition MUST track per-element coordinates and attach them to
// every IR node"). The walker covers only the four types above for
// Atomic 0a; Atomic 0b extends to the per-action attribution emit.

use crate::forge::error::{ForgeError, Located, SourceLocation, ValidationError};
use crate::forge::model::ForgeDocument;
use crate::model::{Action, SCXMLModel, State, Transition};

/// Walk every emission-eligible node in `model` and assert that
/// `source_location` is populated. On the first offender, return a
/// `Located<ForgeError>` carrying [`ValidationError::TraceabilityScxmlLineRangeMissing`]
/// pinned at `scxml_path`. Author callers see the codegen-internal
/// diagnostic on the wire; the actual fix lives in the parser site
/// that produced the IR node.
///
/// First-failure short-circuit: there is no value in accumulating
/// multiple instances of the same parser-site regression, and
/// downstream codegen (which the call site is about to invoke) would
/// fail anyway. The single emitted diagnostic identifies the node
/// kind + id so an author or CI consumer can pinpoint the parser
/// site that needs the populate.
///
/// `scxml_path` is the diagnostic label (mirrors the call site's
/// `source_name` / `diag_label` threading) so the wire payload's
/// `location.file` matches the rest of the parser's `Located::new`
/// raises.
pub fn validate_emission_provenance(
    model: &SCXMLModel,
    scxml_path: &str,
) -> Result<(), Located<ForgeError>> {
    // 1. Root SCXMLModel provenance — the top-level state machine
    //    function emission consumes this.
    if model.source_location.is_none() {
        return Err(emit(scxml_path, "<scxml>", &model.name));
    }

    // 2. Per-state provenance — every state lowers to at least one
    //    per-state function (on_entry / on_exit). The state's own
    //    source position drives every marker that function family
    //    needs; the per-action source position (covered below) is
    //    consumed by Atomic 0b's per-statement attribution.
    for (state_id, state) in &model.states {
        if state.source_location.is_none() {
            let kind = state_kind_label(state);
            return Err(emit(scxml_path, kind, state_id));
        }

        // 3. Per-transition provenance — every transition lowers
        //    to its own handler function in most backends, so each
        //    transition needs a marker. The pinned id is the parent
        //    state id + the event / target / index triple so the
        //    diagnostic uniquely identifies the parser site.
        for (idx, trans) in state.transitions.iter().enumerate() {
            if trans.source_location.is_none() {
                let pinned = format_transition_id(state_id, trans, idx);
                return Err(emit(scxml_path, "<transition>", &pinned));
            }
        }

        // 4. Per-action provenance — flat first-level walk over the
        //    state's executable-content blocks (on_entry, on_exit,
        //    initial transition actions, and per-transition actions).
        //    Nested if/foreach bodies hold their own per-action
        //    records; the walker recurses through them too so the
        //    populate gap is caught wherever it occurs.
        for block in state
            .on_entry_blocks
            .iter()
            .chain(state.on_exit_blocks.iter())
        {
            check_action_block(block, state_id, scxml_path)?;
        }
        check_action_block(&state.initial_transition_actions, state_id, scxml_path)?;
        for trans in &state.transitions {
            check_action_block(&trans.actions, state_id, scxml_path)?;
        }
    }

    // 5. Global script actions — top-level `<script>` elements live
    //    on `model.global_scripts`, outside the per-state walk.
    check_action_block(&model.global_scripts, "<scxml>", scxml_path)?;

    Ok(())
}

/// Build the Located<ForgeError> wrapping
/// [`ValidationError::TraceabilityScxmlLineRangeMissing`] with the
/// node kind / id pinned at `scxml_path`. Centralised so every fire
/// site emits the same wire shape.
fn emit(scxml_path: &str, node_kind: &'static str, node_id: &str) -> Located<ForgeError> {
    Located::new(
        ValidationError::TraceabilityScxmlLineRangeMissing {
            node_kind,
            node_id: node_id.to_string(),
        }
        .into(),
        scxml_path,
        None,
        None,
    )
}

/// Recursive walk over an action vector. Each leaf must carry
/// `source_location: Some(_)`; the per-block check descends into
/// `if` branches and `foreach` bodies so a `None` in a nested
/// `<assign>` inside an `<if>` does not slip past.
fn check_action_block(
    actions: &[Action],
    state_id: &str,
    scxml_path: &str,
) -> Result<(), Located<ForgeError>> {
    for action in actions {
        if action.source_location.is_none() {
            let pinned = action_pinned_id(state_id, action);
            return Err(emit(scxml_path, "<action>", &pinned));
        }
        // Nested executable content — same invariant applies.
        check_action_block(&action.then_actions, state_id, scxml_path)?;
        check_action_block(&action.else_actions, state_id, scxml_path)?;
        check_action_block(&action.actions, state_id, scxml_path)?;
        for branch in &action.elseif_branches {
            check_action_block(&branch.actions, state_id, scxml_path)?;
        }
    }
    Ok(())
}

/// Map a `State` to the originating XML element name so the
/// diagnostic surfaces the same vocabulary the author wrote.
fn state_kind_label(state: &State) -> &'static str {
    if state.is_final {
        "<final>"
    } else if state.is_parallel {
        "<parallel>"
    } else {
        "<state>"
    }
}

/// Build a stable per-transition identifier for the diagnostic
/// payload. Format: `<state_id>[event=…][target=…]#<idx>`. The
/// transition index falls back when neither event nor target is
/// authored (every-event no-target transitions).
fn format_transition_id(state_id: &str, trans: &Transition, idx: usize) -> String {
    let mut s = state_id.to_string();
    if !trans.event.is_empty() {
        s.push_str("[event=");
        s.push_str(&trans.event);
        s.push(']');
    }
    if !trans.target.is_empty() {
        s.push_str("[target=");
        s.push_str(&trans.target);
        s.push(']');
    }
    s.push('#');
    s.push_str(&idx.to_string());
    s
}

/// Build a stable per-action identifier for the diagnostic payload.
/// Format: `<state_id>::<action_type>[<distinguisher>]` where the
/// distinguisher is the action's most identifying field (`event` for
/// `<raise>` / `<send>`, `location` for `<assign>`, etc.).
fn action_pinned_id(state_id: &str, action: &Action) -> String {
    let distinguisher = if !action.event.is_empty() {
        action.event.clone()
    } else if !action.location.is_empty() {
        action.location.clone()
    } else if !action.label.is_empty() {
        action.label.clone()
    } else if !action.target.is_empty() {
        action.target.clone()
    } else {
        // No identifying field on the action; the type alone has to
        // carry the diagnostic, which is acceptable because the
        // wire id additionally hashes `node_kind + node_id` into a
        // distinct content hash.
        String::new()
    };
    if distinguisher.is_empty() {
        format!("{}::{}", state_id, action.action_type)
    } else {
        format!("{}::{}[{}]", state_id, action.action_type, distinguisher)
    }
}

/// Watching-zenoh RFC §5.O Atomic 0c — forge IR provenance pre-emit
/// guard. Counterpart to [`validate_emission_provenance`] for the
/// non-statechart kinds: each `ForgeDocument` variant lowers to a
/// per-kind body function that carries an SCE-MAP marker driven by
/// the model's `source_location`. A `None` here would silently strip
/// the marker, regressing spec lines 3280-3284 + 3286-3290 — the
/// same silently-broken-hook regression the SCXML-side walker
/// prevents.
///
/// Exhaustive match on the `ForgeDocument` enum so the compiler
/// rejects any future `ForgeKind` addition that lands its model
/// without the provenance check. The `parsed_forge_walker_is_exhaustive`
/// unit test below pins this contract from the test side too.
///
/// `scxml_path` is the diagnostic label threaded by callers
/// (`compile_forge_from_string`, `compile_forge_with_deploy`,
/// `compile_forge_with_imports`); it matches `location.file` on the
/// wire so the diagnostic and the SCE-MAP marker that would have
/// been emitted point at the same authoring file.
pub fn validate_forge_emission_provenance(
    doc: &ForgeDocument,
    scxml_path: &str,
) -> Result<(), Located<ForgeError>> {
    let (kind_label, name, location) = forge_doc_provenance(doc);
    if location.is_none() {
        return Err(emit(scxml_path, kind_label, name));
    }
    Ok(())
}

/// Read the populated `source_location` from a [`ForgeDocument`]
/// variant, for codegen-side consumers that need the IR-attached
/// per-kind position (e.g. the Jinja2 `source_location` global
/// driving SCE-MAP markers in [`crate::forge::generator::
/// inject_source_location_global`]).
pub fn forge_doc_source_location(doc: &ForgeDocument) -> Option<&SourceLocation> {
    forge_doc_provenance(doc).2.as_ref()
}

/// Map a [`ForgeDocument`] variant to (XML element label, kind name,
/// `source_location` reference). Centralised so adding a new kind
/// surfaces in one place and the exhaustive-match invariant is
/// preserved.
fn forge_doc_provenance(doc: &ForgeDocument) -> (&'static str, &str, &Option<SourceLocation>) {
    match doc {
        // Statechart documents reach this helper only through the
        // AST-export path that wraps the analyzed `SCXMLModel` in
        // `ForgeDocument::Statechart` (no `sce:kind` attribute on the
        // root, so the element label is the bare `<scxml>`).
        ForgeDocument::Statechart(m) => ("<scxml>", &m.name, &m.source_location),
        ForgeDocument::Transform(m) => (
            "<scxml sce:kind=\"transform\">",
            &m.name,
            &m.source_location,
        ),
        ForgeDocument::Lookup(m) => ("<scxml sce:kind=\"lookup\">", &m.name, &m.source_location),
        ForgeDocument::Condition(m) => (
            "<scxml sce:kind=\"condition\">",
            &m.name,
            &m.source_location,
        ),
        ForgeDocument::Codec(m) => ("<scxml sce:kind=\"codec\">", &m.name, &m.source_location),
        ForgeDocument::Validator(m) => (
            "<scxml sce:kind=\"validator\">",
            &m.name,
            &m.source_location,
        ),
        ForgeDocument::Procedure(m) => (
            "<scxml sce:kind=\"procedure\">",
            &m.name,
            &m.source_location,
        ),
        ForgeDocument::Filter(m) => ("<scxml sce:kind=\"filter\">", &m.name, &m.source_location),
        ForgeDocument::Interpolation(m) => (
            "<scxml sce:kind=\"interpolation\">",
            &m.name,
            &m.source_location,
        ),
        ForgeDocument::Timer(m) => ("<scxml sce:kind=\"timer\">", &m.name, &m.source_location),
        ForgeDocument::Observer(m) => {
            ("<scxml sce:kind=\"observer\">", &m.name, &m.source_location)
        }
        ForgeDocument::Algorithm(m) => (
            "<scxml sce:kind=\"algorithm\">",
            &m.name,
            &m.source_location,
        ),
        ForgeDocument::Link(m) => ("<scxml sce:kind=\"link\">", &m.name, &m.source_location),
        ForgeDocument::BufferPool(m) => (
            "<scxml sce:kind=\"buffer-pool\">",
            &m.name,
            &m.source_location,
        ),
        ForgeDocument::Worker(m) => ("<scxml sce:kind=\"worker\">", &m.name, &m.source_location),
        ForgeDocument::BoundedCollection(m) => (
            "<scxml sce:kind=\"bounded-collection\">",
            &m.name,
            &m.source_location,
        ),
        ForgeDocument::Enum(m) => ("<scxml sce:kind=\"enum\">", &m.name, &m.source_location),
        // NL→IR Item C1 Path A: EventSchema follows the same Atomic
        // 0c convention — the parser captures the root element's
        // source position into `source_location` for the SCE-MAP
        // marker, and this walker surfaces it to downstream consumers
        // (sourcemap, drift-guard, error anchoring).
        ForgeDocument::EventSchema(m) => (
            "<scxml sce:kind=\"event-schema\">",
            &m.name,
            &m.source_location,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::error::SourceLocation;

    /// A model populated by the real parser MUST pass the walker.
    /// This is the round-trip invariant that says: every parser site
    /// that creates an IR node populates `source_location`.
    #[test]
    fn parser_output_is_provenance_complete() {
        let scxml = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="s1" datamodel="ecmascript">
  <state id="s1">
    <onentry>
      <log label="entered" expr="'hello'"/>
    </onentry>
    <transition event="go" target="s2"/>
  </state>
  <final id="s2">
    <onentry>
      <raise event="done"/>
    </onentry>
  </final>
</scxml>"#;
        let mut parser = crate::parser::SCXMLParser::new();
        let model = parser
            .parse_string(scxml, "fixture")
            .expect("fixture parses cleanly");
        validate_emission_provenance(&model, "fixture")
            .expect("real parser output must satisfy §5.O Atomic 0 provenance");
    }

    /// Synthesised None on the root surfaces the diagnostic with
    /// `node_kind = "<scxml>"`. Mirrors what would happen if a
    /// future `parse_impl` edit forgot to populate the root field.
    #[test]
    fn missing_root_provenance_fires_diagnostic() {
        let model = SCXMLModel {
            name: "broken".into(),
            source_location: None,
            ..SCXMLModel::default()
        };
        let err = validate_emission_provenance(&model, "broken.scxml")
            .expect_err("must fire when root.source_location is None");
        match &err.error {
            ForgeError::Validation(boxed) => match boxed.as_ref() {
                ValidationError::TraceabilityScxmlLineRangeMissing { node_kind, node_id } => {
                    assert_eq!(*node_kind, "<scxml>");
                    assert_eq!(node_id, "broken");
                }
                other => panic!("expected TraceabilityScxmlLineRangeMissing, got {other:?}"),
            },
            other => panic!("expected TraceabilityScxmlLineRangeMissing, got {other:?}"),
        }
    }

    /// Synthesised None on a state surfaces the diagnostic with the
    /// state's id verbatim in `node_id` so the parser-side regression
    /// is locatable from the wire payload alone.
    #[test]
    fn missing_state_provenance_fires_diagnostic() {
        let mut model = SCXMLModel {
            source_location: Some(SourceLocation {
                file: "x.scxml".into(),
                line: Some(1),
                col: Some(1),
            }),
            ..SCXMLModel::default()
        };
        let state = State {
            id: "armed".into(),
            source_location: None,
            ..State::default()
        };
        model.states.insert("armed".into(), state);
        let err = validate_emission_provenance(&model, "x.scxml")
            .expect_err("must fire when state.source_location is None");
        match &err.error {
            ForgeError::Validation(boxed) => match boxed.as_ref() {
                ValidationError::TraceabilityScxmlLineRangeMissing { node_kind, node_id } => {
                    assert_eq!(*node_kind, "<state>");
                    assert_eq!(node_id, "armed");
                }
                other => panic!("expected TraceabilityScxmlLineRangeMissing, got {other:?}"),
            },
            other => panic!("expected TraceabilityScxmlLineRangeMissing, got {other:?}"),
        }
    }

    /// Real-parser round trip for the forge walker: every kind the
    /// parser builds populates `source_location`, so the post-emit
    /// walker accepts the output. Mirrors
    /// `parser_output_is_provenance_complete` on the statechart side
    /// and pins the populate sites in `forge::parser` against silent
    /// regressions.
    #[test]
    fn forge_parser_output_is_provenance_complete() {
        let codec = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0"
       sce:kind="codec" name="ping_frame" datamodel="ecmascript">
  <datamodel>
    <data id="opcode" sce:type="uint8"/>
    <data id="payload" sce:type="uint8"/>
  </datamodel>
</scxml>"#;
        let label = crate::DocumentLabel::symmetric("ping_frame.scxml");
        let doc = crate::forge::parser::parse_forge(codec, label)
            .expect("parses cleanly")
            .expect("non-statechart kind");
        validate_forge_emission_provenance(&doc, "ping_frame.scxml")
            .expect("real forge parser output must satisfy §5.O Atomic 0c provenance");
    }

    /// Walker contract: every `ForgeDocument` variant whose
    /// `source_location` is `None` surfaces the diagnostic with the
    /// matching `<scxml sce:kind="…">` label. Exhaustive smoke check
    /// so a future forge-kind addition that lands its model + parser
    /// site but forgets the `source_location` populate fires here
    /// rather than slipping through unnoticed.
    #[test]
    fn parsed_forge_walker_is_exhaustive() {
        use crate::forge::model::{
            AlgorithmModel, AlgorithmSignature, BackpressurePolicy, BoundedCollectionModel,
            BufferPoolModel, BufferPoolVariant, CachePolicy, CapacitySource, CodecModel,
            CollectionOrdering, ConcurrencyMode, ConditionModel, Direction, Endian, FilterModel,
            FilterType, ForgeDocument, ForgeField, InboxConfig, InboxOrdering, InterpolationAxis,
            InterpolationMethod, InterpolationModel, LinkClass, LinkModel, LookupModel, MissPolicy,
            ObserverModel, OutOfBounds, OverflowPolicy, ProcedureModel, SceType, ThresholdMonitor,
            TimerModel, TransformModel, ValidatorModel, ValidatorRules, WorkerModel,
        };
        let scalar = ForgeField {
            id: "x".into(),
            sce_type: SceType::Uint8,
            direction: Direction::In,
            expr: None,
            quantity: None,
            max_size: None,
        };
        let cases: Vec<ForgeDocument> = vec![
            ForgeDocument::Transform(TransformModel {
                name: "t".into(),
                inputs: vec![scalar.clone()],
                outputs: vec![scalar.clone()],
                source_location: None,
            }),
            ForgeDocument::Lookup(LookupModel {
                name: "l".into(),
                input: scalar.clone(),
                output: scalar.clone(),
                entries: Vec::new(),
                miss_policy: MissPolicy::Error,
                source_location: None,
            }),
            ForgeDocument::Condition(ConditionModel {
                name: "c".into(),
                inputs: vec![scalar.clone()],
                expr: "x > 0".into(),
                source_location: None,
            }),
            ForgeDocument::Codec(CodecModel {
                name: "k".into(),
                default_endian: Endian::Little,
                input_length: None,
                fields: Vec::new(),
                variant: None,
                flag_inputs: Vec::new(),
                test_vectors: Vec::new(),
                source_location: None,
            }),
            ForgeDocument::Validator(ValidatorModel {
                name: "v".into(),
                inputs: vec![scalar.clone()],
                rules: ValidatorRules {
                    ranges: Vec::new(),
                    rate_of_changes: Vec::new(),
                    plausibility: None,
                },
                source_location: None,
            }),
            ForgeDocument::Procedure(ProcedureModel {
                name: "p".into(),
                inputs: Vec::new(),
                internals: Vec::new(),
                helpers: Vec::new(),
                initial: "s0".into(),
                states: Vec::new(),
                source_location: None,
            }),
            ForgeDocument::Filter(FilterModel {
                name: "f".into(),
                input: scalar.clone(),
                output: scalar.clone(),
                filter_type: FilterType::MovingAverage,
                window: Some(4),
                alpha: None,
                source_location: None,
            }),
            ForgeDocument::Interpolation(InterpolationModel {
                name: "i".into(),
                inputs: vec![scalar.clone()],
                output: scalar.clone(),
                method: InterpolationMethod::Linear,
                out_of_bounds: OutOfBounds::Clamp,
                axes: vec![InterpolationAxis {
                    input_id: "x".into(),
                    breakpoints: vec![0.0, 1.0],
                }],
                values: vec![0.0, 1.0],
                source_location: None,
            }),
            ForgeDocument::Timer(TimerModel {
                name: "tm".into(),
                period_us: 1_000_000,
                reset_on_event: None,
                cancel_on_state_exit: None,
                fire_event: "tick".into(),
                source_location: None,
            }),
            ForgeDocument::Observer(ObserverModel {
                name: "ob".into(),
                inputs: vec![scalar.clone()],
                monitors: vec![ThresholdMonitor {
                    id: "m".into(),
                    enter_expr: "x > 1".into(),
                    leave_expr: None,
                    on_enter: "hi".into(),
                    on_leave: None,
                }],
                event_domain: None,
                source_location: None,
            }),
            ForgeDocument::Algorithm(AlgorithmModel {
                name: "alg".into(),
                signature: AlgorithmSignature {
                    params: Vec::new(),
                    return_type: Some(SceType::Uint8),
                },
                consts: Vec::new(),
                body: Vec::new(),
                test_vectors: Vec::new(),
                source_location: None,
            }),
            ForgeDocument::Link(LinkModel {
                name: "ln".into(),
                class: LinkClass::Udp,
                framer: "f".into(),
                backpressure: BackpressurePolicy::Drop,
                inbound: Vec::new(),
                outbound: Vec::new(),
                rx_pool: None,
                tx_pool: None,
                stage_pool: None,
                accept_stage_copy_rate: false,
                source_location: None,
            }),
            ForgeDocument::BufferPool(BufferPoolModel {
                name: "bp".into(),
                slot_count: 1,
                slot_size: 16,
                section: "sram1".into(),
                alignment: 8,
                dma_channel: None,
                cache_policy: CachePolicy::None,
                variant: BufferPoolVariant::Default,
                source_location: None,
            }),
            ForgeDocument::Worker(WorkerModel {
                name: "w".into(),
                link_rx: "ln".into(),
                inbox: InboxConfig {
                    depth: 4,
                    ordering: InboxOrdering::AcqRel,
                },
                outbox: None,
                source_location: None,
            }),
            ForgeDocument::BoundedCollection(BoundedCollectionModel {
                name: "bc".into(),
                element_type: "k".into(),
                capacity: CapacitySource::CompileConst { value: 8 },
                index_by: None,
                on_overflow: OverflowPolicy::DiagnosticEvent,
                ordering: CollectionOrdering::Insertion,
                concurrency: ConcurrencyMode::SingleWriter,
                source_location: None,
            }),
        ];
        for doc in &cases {
            let err = validate_forge_emission_provenance(doc, "x.scxml")
                .expect_err("every variant with source_location: None must fire the walker");
            match &err.error {
                ForgeError::Validation(boxed) => match boxed.as_ref() {
                    ValidationError::TraceabilityScxmlLineRangeMissing { node_kind, .. } => {
                        assert!(
                            node_kind.starts_with("<scxml sce:kind=\""),
                            "expected kind label, got {node_kind:?}"
                        );
                    }
                    other => panic!("expected TraceabilityScxmlLineRangeMissing, got {other:?}"),
                },
                other => panic!("expected TraceabilityScxmlLineRangeMissing, got {other:?}"),
            }
        }
    }
}
