// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which Event I/O Processor and `<invoke>` types this build has a
//! lowering path for — and the sites that name one it does not.
//!
//! Two things live here because they are one fact read two ways.
//!
//! **The accepted set.** [`is_supported_send_type`] is the single place
//! the set is spelled. It used to be spelled five more times, as a
//! literal list inside the Rust, Go, Python, Kotlin and C11 send
//! templates, so adding a processor meant editing six files and a
//! backend that was missed diverged silently — the generated code is
//! the only place the difference shows, and no test reads six templates
//! against each other. The templates now read
//! [`crate::model::Action::send_type_unsupported`], which this module
//! decides.
//!
//! **The sites.** A document may legitimately name a type no build
//! implements: the specification defines that case rather than leaving
//! it undefined, so such a document is valid SCXML with defined meaning
//! (see [`crate::model::UnsupportedInvokeInfo`] for the same argument on
//! the invoke side). The generated code is therefore correct to accept
//! it and raise at runtime — but until this module existed, that
//! decision was reached at build time and then discarded. A consumer
//! compiled green, passed every test that did not enter the state, and
//! met the refusal hours later in production.
//!
//! So the answer is published rather than enforced. [`analyze`] returns
//! the sites, `sce-codegen` projects them onto the stdout manifest as
//! `needs_host_processor` + `host_processor_causes`, and a build that
//! wants to fail on them can — while a build deliberately relying on the
//! runtime refusal keeps working unchanged. Rejecting here instead would
//! refuse a document the specification permits.
//!
//! Scope, stated because the check is narrower than the sentence a
//! reader would infer: only a **literal** `type` is judged. A `typeexpr`
//! resolves at runtime and no build-time walk can name its value, so
//! such a site is absent from the cause list and is refused by the
//! generated code's own dynamic check.

use crate::forge::error::SourceLocation;
use crate::model::{Action, Invoke, SCXMLModel, State};

/// SCXML Event I/O Processor URI (§scxml-C-1) — the default when
/// `<send>` carries no `type`.
pub const SCXML_EVENT_PROCESSOR_TYPE: &str = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor";

/// BasicHTTP Event I/O Processor URI (§scxml-C-2).
pub const BASIC_HTTP_EVENT_PROCESSOR_TYPE: &str =
    "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor";

/// Every `<send type="...">` value this build lowers to a delivery path.
///
/// Ordered as the specification introduces them. Exposed as a slice
/// rather than kept private so a test can assert the runtime-side
/// spelling in `sce/include/common/SendHelper.h` still names the same
/// set — the build decides at codegen time and the Interpreter decides
/// at evaluation time, and the two answering differently for one URI is
/// exactly the divergence this constant exists to make visible.
pub const SUPPORTED_SEND_TYPES: &[&str] =
    &[SCXML_EVENT_PROCESSOR_TYPE, BASIC_HTTP_EVENT_PROCESSOR_TYPE];

/// Whether `send_type` names an Event I/O Processor this build can
/// deliver through.
///
/// An empty string is the absent attribute, which §scxml-6.2 defines as
/// the SCXML Event I/O Processor, so it is supported. Every other value
/// outside [`SUPPORTED_SEND_TYPES`] is one the generated code refuses at
/// runtime.
pub fn is_supported_send_type(send_type: &str) -> bool {
    // §scxml-6.2: "If the SCXML Processor does not support the type that
    // is specified, it MUST place the event error.execution on the
    // internal event queue." Deciding membership is what makes that
    // refusal reachable; deciding it HERE is what lets the decision also
    // be reported instead of only performed.
    send_type.is_empty() || SUPPORTED_SEND_TYPES.contains(&send_type)
}

/// One site naming a processor type this build has no path for.
///
/// Separate from the wire record for the reason
/// [`crate::script_engine_analyzer::ScriptEngineCauseKind`] is: the
/// variant is the reviewable enumeration, and `kind` on the wire is a
/// stable kebab-case token that does not move when the variant is
/// renamed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostProcessorCauseKind {
    /// `<send type="...">` whose literal names no Event I/O Processor
    /// this build implements.
    SendType {
        /// Owning state, so the report names where the send lives.
        state_id: String,
        /// The `type` attribute verbatim.
        processor_type: String,
    },
    /// `<invoke type="...">` whose literal names no invoker this build
    /// implements.
    InvokeType {
        /// Owning state.
        state_id: String,
        /// `<invoke id="...">` — present for every invoke, auto-derived
        /// when the author wrote none.
        invoke_id: String,
        /// The `type` attribute verbatim.
        processor_type: String,
    },
}

/// A [`HostProcessorCauseKind`] together with where it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostProcessorCause {
    pub kind: HostProcessorCauseKind,
    pub location: Option<SourceLocation>,
}

impl HostProcessorCause {
    fn new(kind: HostProcessorCauseKind, location: Option<&SourceLocation>) -> Self {
        Self {
            kind,
            location: location.cloned(),
        }
    }

    /// Project onto the manifest wire shape. Exhaustive by construction —
    /// adding a variant without choosing its wire `kind` does not compile.
    pub fn to_wire(&self) -> HostProcessorCauseRecord {
        use HostProcessorCauseKind as K;
        let base = match &self.kind {
            K::SendType {
                state_id,
                processor_type,
            } => HostProcessorCauseRecord {
                kind: "send-type",
                processor_type: processor_type.clone(),
                state: Some(state_id.clone()),
                invoke: None,
                location: None,
            },
            K::InvokeType {
                state_id,
                invoke_id,
                processor_type,
            } => HostProcessorCauseRecord {
                kind: "invoke-type",
                processor_type: processor_type.clone(),
                state: Some(state_id.clone()),
                invoke: Some(invoke_id.clone()),
                location: None,
            },
        };
        HostProcessorCauseRecord {
            location: self.location.clone(),
            ..base
        }
    }
}

/// Wire projection of one [`HostProcessorCause`] — the shape the
/// `sce-codegen` stdout manifest carries in `host_processor_causes`.
///
/// `needs_host_processor` on its own tells a consumer that some site in
/// the document will refuse at runtime, but not which one. A build
/// gating on the flag would then fail with nothing to act on; these
/// records name the element and the URI, so the gate can point at a line
/// of SCXML.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct HostProcessorCauseRecord {
    /// Stable kebab-case discriminator: `send-type` or `invoke-type`.
    /// Consumers dispatch on this and must tolerate unknown values.
    pub kind: &'static str,
    /// The `type` attribute verbatim, so the report names the URI that
    /// was refused rather than only the element that carried it.
    pub processor_type: String,
    /// Owning state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// `<invoke id="…">`, for the invoke-anchored cause.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoke: Option<String>,
    /// Where in the source. Same `{file, line, col}` shape a diagnostic
    /// carries, so tooling anchors this exactly as it anchors a
    /// rejection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<SourceLocation>,
}

/// Walk `model` and return every site naming a processor type this build
/// has no lowering path for.
///
/// Empty iff [`needs_host_processor`] would return `false`.
pub fn analyze(model: &SCXMLModel) -> Vec<HostProcessorCause> {
    let mut causes = Vec::new();
    for (state_id, state) in &model.states {
        collect_state_causes(state_id, state, &mut causes);
    }
    // `model.states` is keyed by state id, so the walk above visits
    // states alphabetically — a document whose `<send>` is written above
    // its `<invoke>` would be reported the other way round. This list is
    // read by a person looking for the line to open, so it is ordered by
    // that line. Sites with no location sort last rather than first:
    // absent position is not position zero.
    causes.sort_by_key(|c| {
        let (line, col) = c
            .location
            .as_ref()
            .map(|l| (l.line.unwrap_or(0), l.col.unwrap_or(0)))
            .unwrap_or((u32::MAX, u32::MAX));
        (line, col)
    });
    causes
}

/// `true` iff `analyze(model)` would return any cause.
///
/// Thin wrapper for callers that only need the answer — `analyze` is the
/// single traversal, and spelling `.is_empty()` at each call site is how
/// a second, subtly different predicate gets written.
pub fn needs_host_processor(model: &SCXMLModel) -> bool {
    !analyze(model).is_empty()
}

fn collect_state_causes(state_id: &str, state: &State, out: &mut Vec<HostProcessorCause>) {
    for trans in &state.transitions {
        for action in &trans.actions {
            collect_action_causes(state_id, action, out);
        }
    }
    for block in state
        .on_entry_blocks
        .iter()
        .chain(state.on_exit_blocks.iter())
    {
        for action in block {
            collect_action_causes(state_id, action, out);
        }
    }
    for action in &state.initial_transition_actions {
        collect_action_causes(state_id, action, out);
    }
    for action in &state.initial_history_default_actions {
        collect_action_causes(state_id, action, out);
    }
    for invoke in &state.invokes {
        collect_invoke_causes(state_id, invoke, out);
    }
}

/// Recurse through the containers that hold executable content, so a
/// `<send>` nested in `<if>` / `<foreach>` is reported like a top-level
/// one. A walk that only read the block's own actions would report the
/// flat case and stay silent on the nested one — the same document,
/// two answers.
fn collect_action_causes(state_id: &str, action: &Action, out: &mut Vec<HostProcessorCause>) {
    if action.send_type_unsupported {
        out.push(HostProcessorCause::new(
            HostProcessorCauseKind::SendType {
                state_id: state_id.to_string(),
                processor_type: action.send_type.clone(),
            },
            action.source_location.as_ref(),
        ));
    }
    // `actions` is `<foreach>`'s body; `then_actions` / `else_actions` /
    // each `elseif_branches` entry are `<if>`'s. Chained rather than
    // matched on `action_type` so a container added later is walked by
    // default — the failure mode of the other spelling is silence.
    for nested in action
        .actions
        .iter()
        .chain(action.then_actions.iter())
        .chain(action.else_actions.iter())
        .chain(action.elseif_branches.iter().flat_map(|b| b.actions.iter()))
    {
        collect_action_causes(state_id, nested, out);
    }
}

fn collect_invoke_causes(state_id: &str, invoke: &Invoke, out: &mut Vec<HostProcessorCause>) {
    // §scxml-6.4.1: an `<invoke>` naming a type the platform does not
    // support raises `error.execution` and starts no child. The parser
    // already classified this site into `Invoke::Unsupported` so the
    // runtime observable exists; reading that classification here is what
    // gives the build one too.
    let Invoke::Unsupported(info) = invoke else {
        return;
    };
    out.push(HostProcessorCause::new(
        HostProcessorCauseKind::InvokeType {
            state_id: state_id.to_string(),
            invoke_id: info.base.invoke_id.clone(),
            processor_type: info.invoke_type.clone(),
        },
        info.base.source_location.as_ref(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    fn parse(scxml: &str) -> SCXMLModel {
        SCXMLParser::new().parse_string(scxml, "test").unwrap()
    }

    /// Wrap `body` in the smallest document that parses, so each test
    /// reads as the one construct it is about.
    fn doc(body: &str) -> String {
        format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">{body}</scxml>"#
        )
    }

    #[test]
    fn the_two_types_the_specification_names_are_supported() {
        assert!(is_supported_send_type(SCXML_EVENT_PROCESSOR_TYPE));
        assert!(is_supported_send_type(BASIC_HTTP_EVENT_PROCESSOR_TYPE));
    }

    /// §scxml-6.2 makes an absent `type` the SCXML Event I/O Processor,
    /// so the empty string is the default and not an unknown value. Read
    /// the other way this is the false-positive guard: every `<send>`
    /// without a `type` attribute reaches this predicate, so getting it
    /// wrong would report the whole corpus.
    #[test]
    fn an_absent_type_is_the_default_processor_not_an_unknown_one() {
        assert!(is_supported_send_type(""));
        let model = parse(&doc(
            r#"<state id="s"><onentry><send event="e"/></onentry></state>"#,
        ));
        assert!(analyze(&model).is_empty());
        assert!(!needs_host_processor(&model));
    }

    #[test]
    fn a_type_outside_the_set_is_not_supported() {
        assert!(!is_supported_send_type("x-sprag-host"));
        // Near-misses, because a substring or prefix comparison would
        // accept these and no fixture in the corpus would notice.
        assert!(!is_supported_send_type("http://www.w3.org/TR/scxml/#"));
        assert!(!is_supported_send_type(
            "http://www.w3.org/TR/scxml/#SCXMLEventProcessorX"
        ));
    }

    #[test]
    fn an_unsupported_send_type_is_reported_with_its_state_and_uri() {
        let model = parse(&doc(
            r#"<state id="s"><onentry><send type="x-sprag-host" event="e"/></onentry></state>"#,
        ));
        let causes = analyze(&model);
        assert_eq!(causes.len(), 1, "{causes:?}");
        assert_eq!(
            causes[0].kind,
            HostProcessorCauseKind::SendType {
                state_id: "s".to_string(),
                processor_type: "x-sprag-host".to_string(),
            }
        );
        assert!(needs_host_processor(&model));
    }

    #[test]
    fn an_unsupported_invoke_type_is_reported_with_its_invoke_id() {
        let model = parse(&doc(
            r#"<state id="s"><invoke id="probe" type="x-sprag-host"/></state>"#,
        ));
        let causes = analyze(&model);
        assert_eq!(causes.len(), 1, "{causes:?}");
        assert_eq!(
            causes[0].kind,
            HostProcessorCauseKind::InvokeType {
                state_id: "s".to_string(),
                invoke_id: "probe".to_string(),
                processor_type: "x-sprag-host".to_string(),
            }
        );
    }

    /// A `<send>` inside `<if>` / `<foreach>` refuses at runtime exactly
    /// as a top-level one does, so a walk that only read the block's own
    /// actions would give the same document two answers depending on
    /// where the author put it.
    #[test]
    fn a_send_nested_in_executable_content_is_still_reported() {
        let model = parse(&doc(r#"<state id="s"><onentry>
                 <if cond="true"><send type="x-in-then" event="a"/>
                 <else/><send type="x-in-else" event="b"/></if>
                 <foreach array="xs" item="x"><send type="x-in-foreach" event="c"/></foreach>
               </onentry></state>"#));
        let found: Vec<String> = analyze(&model)
            .iter()
            .map(|c| match &c.kind {
                HostProcessorCauseKind::SendType { processor_type, .. } => processor_type.clone(),
                other => panic!("expected a send cause, got {other:?}"),
            })
            .collect();
        assert_eq!(found, vec!["x-in-then", "x-in-else", "x-in-foreach"]);
    }

    /// The list is read by a person looking for the line to open, so it
    /// runs down the document. `model.states` is keyed by state id, so
    /// without the sort this document reports `later` before `earlier`.
    #[test]
    fn causes_are_ordered_by_source_position_not_by_state_id() {
        let model = parse(&doc(
            r#"<state id="zebra"><onentry><send type="x-first" event="a"/></onentry>
                 <transition event="go" target="alpha"/></state>
               <state id="alpha"><onentry><send type="x-second" event="b"/></onentry></state>"#,
        ));
        let causes = analyze(&model);
        let lines: Vec<u32> = causes
            .iter()
            .map(|c| c.location.as_ref().and_then(|l| l.line).unwrap_or(0))
            .collect();
        let types: Vec<&str> = causes
            .iter()
            .map(|c| match &c.kind {
                HostProcessorCauseKind::SendType { processor_type, .. } => processor_type.as_str(),
                other => panic!("expected send causes, got {other:?}"),
            })
            .collect();
        assert_eq!(types, vec!["x-first", "x-second"], "lines were {lines:?}");
        assert!(lines[0] <= lines[1], "not ordered by line: {lines:?}");
    }

    /// The model flag every backend's send template reads must agree
    /// with the predicate — they are one decision, and a template
    /// reading a flag the analyzer did not set would emit a send where
    /// the report promised a refusal.
    #[test]
    fn the_template_flag_agrees_with_the_predicate() {
        let model = parse(&doc(r#"<state id="s"><onentry>
                 <send event="default"/>
                 <send type="http://www.w3.org/TR/scxml/#SCXMLEventProcessor" event="named"/>
                 <send type="x-sprag-host" event="refused"/>
               </onentry></state>"#));
        let flags: Vec<bool> = model.states["s"].on_entry_blocks[0]
            .iter()
            .map(|a| a.send_type_unsupported)
            .collect();
        assert_eq!(flags, vec![false, false, true]);
        for action in &model.states["s"].on_entry_blocks[0] {
            assert_eq!(
                action.send_type_unsupported,
                !action.send_type.is_empty() && !is_supported_send_type(&action.send_type),
                "flag disagrees with the predicate for type {:?}",
                action.send_type,
            );
        }
    }

    /// The wire `kind` tokens are the consumer's dispatch key. Renaming
    /// a Rust variant must not move them, which only holds if something
    /// reads them.
    #[test]
    fn wire_kinds_are_the_documented_tokens() {
        let send = HostProcessorCause::new(
            HostProcessorCauseKind::SendType {
                state_id: "s".into(),
                processor_type: "x".into(),
            },
            None,
        );
        let invoke = HostProcessorCause::new(
            HostProcessorCauseKind::InvokeType {
                state_id: "s".into(),
                invoke_id: "i".into(),
                processor_type: "x".into(),
            },
            None,
        );
        assert_eq!(send.to_wire().kind, "send-type");
        assert_eq!(invoke.to_wire().kind, "invoke-type");
        // The URI is what a reader acts on; an empty one would make the
        // record name only the element.
        assert_eq!(send.to_wire().processor_type, "x");
        assert_eq!(invoke.to_wire().invoke.as_deref(), Some("i"));
    }

    /// The set is decided here at build time and again by
    /// `SendHelper::isSupportedSendType` at Interpreter evaluation time
    /// — two engines answering one question. They are allowed to be two
    /// implementations; they are not allowed to be two answers, and
    /// nothing but this test reads them against each other.
    #[test]
    fn the_cpp_runtime_spelling_names_the_same_set() {
        let header = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../sce/include/common/SendHelper.h"
        ))
        .expect("the C++ send helper is part of this workspace");
        let body = header
            .split_once("static bool isSupportedSendType(")
            .expect("SendHelper still declares isSupportedSendType")
            .1;
        let body = &body[..body.find("\n    }").expect("the function is brace-closed")];
        for accepted in SUPPORTED_SEND_TYPES {
            // The SCXML processor URI reaches the C++ side through a
            // named constant rather than a literal, so accept either
            // spelling — what is being pinned is that the URI is
            // reachable from that function, not how it is written.
            let named_constant = accepted
                .rsplit_once('#')
                .map(|(_, frag)| body.contains(frag))
                .unwrap_or(false);
            assert!(
                body.contains(accepted) || named_constant,
                "the build accepts {accepted} but the C++ runtime check does not name it",
            );
        }
    }
}
