// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What a deployment decides, in the form the review surface prints.
//!
//! [`crate::forge::pseudo`] renders a document; a deployment changes
//! what that document's `<send>` elements actually do without changing
//! a byte of it. This module is the bridge: it runs the mesh pipeline
//! and hands back a [`Deployment`] of plain facts, so the renderer
//! stays free of every mesh type and a caller can render a review
//! surface without one.
//!
//! # ⚠ The resolution comes from the pipeline, not from a second read
//!
//! [`deployment_for`] calls [`crate::compile_mesh_transport`] and takes
//! `MeshResult::resolved_targets` — the same values the templates were
//! handed. Reading `deploy.yaml` again here and working out the
//! bindings would be quicker and would be a second stage sequence: it
//! could disagree with codegen about precisely the thing a reviewer is
//! being asked to approve, and nothing would catch the disagreement
//! because both would look reasonable. The cost of running codegen and
//! discarding its output is the price of there being one answer.
//!
//! # ⚠ The model the pipeline sees is not the model that is rendered
//!
//! `compile_mesh_transport` takes `&mut SCXMLModel` and injects into
//! it: SCE_MESH.md §13 auto-symmetry adds `<onexit>` unsubscribes, and
//! Session E server detection adds the response leg of each detected
//! RPC pair. Those sends are the deployment's, not the author's. So
//! this module runs the pipeline on a CLONE and leaves the caller's
//! model untouched — the rendering must be of what the author wrote,
//! with everything derived carried as facts beside it.
//!
//! ⚠⚠ What that leaves open, stated rather than hidden: the injected
//! sends themselves are **not yet** in the rendering. A reviewer sees
//! every binding for every send the author wrote, and does not see a
//! send only the deployment writes. [`injected_send_count`] is what
//! the facts say about it until the sends themselves are rendered.
//!
//! # The facts are derived from the types, not listed here
//!
//! [`crate::mesh::topology::ResolvedTarget`] is what the deployment
//! settled for one target, and a field of it that reaches no reviewer
//! is a binding approved unseen. So every type reachable from it that
//! carries a `skip_serializing_if` — `ResolvedTarget` itself,
//! [`crate::mesh::topology::TransportState`],
//! [`crate::mesh::topology::EventPatternInfo`] and
//! [`crate::mesh::deploy::AuthPolicyConfig`] — is taken apart by name
//! with no `..` and no `_` arm, and the compiler is the check: a field
//! or variant added to any of them stops the build here until somebody
//! decides what a reviewer should be told.
//!
//! ⚠ Serialising and flattening was the first shape of this, and it
//! was wrong in a way that does not show. Six fields of
//! `ResolvedTarget` alone skip when they are empty, so `retry: None`
//! printed nothing at all — and on a page, "there is no retry policy"
//! and "retry is not a thing here" read identically. The serialised
//! form is tuned for what the templates probe with `is defined`; it
//! does not owe a reviewer an answer for every field.
//!
//! Everything else still goes through serde, which is sound only while
//! nothing else reachable skips — and that is held by
//! `nothing_the_builder_leaves_to_serde_can_skip_a_field`, not by this
//! paragraph. ⚠ It has to be: `AuthPolicyConfig` is on the list above
//! because that gate found it, after three separate readings of the
//! sources by hand had each concluded there was nothing there.

use std::path::Path;

use crate::forge::pseudo::{Deployment, Fact};
use crate::generator::Language;
use crate::mesh::error::MeshError;
use crate::mesh::pattern::CommunicationPattern;
use crate::mesh::topology::{EventPatternInfo, ResolvedTarget, TransportState};
use crate::model::SCXMLModel;

/// The facts a deployment settles about this machine.
///
/// `model` is read, never written: the pipeline runs against a clone.
/// The returned [`Deployment`] is keyed by the target exactly as the
/// document writes it, which is how the renderer finds the `to` clause
/// to put each fact under.
pub fn deployment_for(
    model: &SCXMLModel,
    deploy_path: &Path,
    language: Language,
) -> Result<Deployment, MeshError> {
    let mut scratch = model.clone();
    let result = crate::compile_mesh_transport(&mut scratch, deploy_path, language)?;

    let mut deployment = Deployment::default();

    if let Some(device) = &result.device {
        deployment.machine.push(Fact::new("device", device));
    }

    // A `targetexpr` is resolved at run time, so the deployment has
    // nothing to bind and the renderer has no `to` clause to hang a
    // fact on. Said at machine level instead of left as an absence: "no
    // annotation" and "could not be annotated" look identical on the
    // page, and only one of them is a reason to look closer.
    for w in &result.dynamic_target_warnings {
        deployment.machine.push(Fact::new(
            "unresolved-target",
            format!("{} in state {}", w.targetexpr, w.state),
        ));
    }

    let injected = injected_send_count(model, &scratch);
    if injected > 0 {
        deployment
            .machine
            .push(Fact::new("sends-injected-not-shown", injected.to_string()));
    }

    for target in &result.resolved_targets {
        let key = target.target.as_str().to_string();
        deployment.targets.insert(key, facts_for(target));
    }

    Ok(deployment)
}

/// How many `<send>` actions the pipeline added to the model.
///
/// Counted by walking both models rather than by trusting a stage to
/// report what it injected: `MeshResult` carries `auto_subscriptions`
/// and nothing for the Session E response legs, so a report-based count
/// would be short by exactly the sends nobody remembered to announce.
/// A count from the two models is short by nothing.
///
/// The walk is [`crate::mesh::topology::for_each_send_action`], the same
/// one the resolver uses, so "a send" means one thing here and there.
pub fn injected_send_count(authored: &SCXMLModel, deployed: &SCXMLModel) -> usize {
    sends_in(deployed).saturating_sub(sends_in(authored))
}

fn sends_in(model: &SCXMLModel) -> usize {
    let mut n = 0usize;
    crate::mesh::topology::for_each_send_action(model, |_, _| n += 1);
    n
}

/// One fact per thing the deployment settled for this target.
///
/// # ⚠ Destructured without `..`, and that is the totality check
///
/// Serialising the whole `ResolvedTarget` and flattening it was the
/// first shape of this function, and it was wrong in a way that does
/// not show: five of the eleven fields carry
/// `skip_serializing_if`, so `retry: None` and an empty
/// `subscription_events` produced no line at all. On a review surface
/// "there is no retry policy" and "retry is not a thing here" then read
/// identically, and only one of them is true. The serialised shape is
/// tuned for what the templates probe with `is defined`; it is not a
/// shape that owes a reviewer an answer for every field.
///
/// So the fields are taken apart by name with no `..` arm. A field
/// added to [`ResolvedTarget`] stops the build here until somebody
/// decides what a reviewer should be told about it — the same
/// discipline the renderer's per-kind `match` uses, and the only kind
/// of totality check that cannot quietly stop working.
fn facts_for(target: &ResolvedTarget) -> Vec<Fact> {
    // ⚠ No `..`. See above.
    let ResolvedTarget {
        // The key this annotation is printed under; repeating it says
        // nothing a reader cannot see one line up.
        target: _,
        events,
        event_patterns,
        subscription_events,
        state,
        invoke_sites,
        ordering,
        responders,
        retry,
        auth,
        pool_plan,
    } = target;

    let mut facts = Vec::new();
    // `transport` rather than the field's own name `state`: the field
    // name is Rust's and the reviewer is reading about a deployment.
    push_transport("transport", state, &mut facts);
    push_list("events", events, &mut facts);
    if event_patterns.is_empty() {
        facts.push(Fact::new("event-pattern", "(none)"));
    }
    for (i, p) in event_patterns.iter().enumerate() {
        push_event_pattern(&format!("event-pattern.{i}"), p, &mut facts);
    }
    push_list("subscribes", subscription_events, &mut facts);
    push_nested_list("invoke", invoke_sites, &mut facts);
    push_nested("ordering", ordering, &mut facts);
    push_list("responders", responders, &mut facts);
    push_optional("retry", retry.as_ref(), &mut facts);
    match auth {
        Some(a) => push_auth("auth", a, &mut facts),
        None => facts.push(Fact::new("auth", "(none)")),
    }
    push_optional("pool", pool_plan.as_ref(), &mut facts);
    facts
}

/// What the deployment settled about the transport, in full.
///
/// ⚠ A `match` with no `_` arm and no `..` in any pattern, for the
/// reason [`facts_for`] destructures: `TransportState::Shm` carries two
/// `skip_serializing_if` fields, so the serialised form of a shm
/// binding with default capacities says nothing about them, and "the
/// default arena" then reads exactly like "this transport has no
/// arena". A variant added to the enum stops the build here.
fn push_transport(name: &str, state: &TransportState, out: &mut Vec<Fact>) {
    // Which transport it is comes from the declared namer, once.
    out.push(Fact::new(name, state.transport_name()));
    let at = |suffix: &str| format!("{name}.{suffix}");
    match state {
        TransportState::Local => {}
        TransportState::Shm {
            arena_bytes,
            ring_capacity,
        } => {
            push_optional_scalar(&at("arena-bytes"), arena_bytes.as_ref(), out);
            push_optional_scalar(&at("ring-capacity"), ring_capacity.as_ref(), out);
        }
        TransportState::Someip {
            service,
            event_bindings,
            extra,
        } => {
            push_nested(&at("service"), service, out);
            push_nested(&at("event-binding"), event_bindings, out);
            push_nested(&at("extra"), extra, out);
        }
        TransportState::Zenoh { key, extra } => {
            out.push(Fact::new(at("key"), key));
            push_nested(&at("extra"), extra, out);
        }
        TransportState::CustomTcp { connect, extra } => {
            out.push(Fact::new(at("connect"), connect));
            push_nested(&at("extra"), extra, out);
        }
        TransportState::Dds { topic, extra } => {
            out.push(Fact::new(at("topic"), topic));
            push_nested(&at("extra"), extra, out);
        }
        TransportState::Unimplemented { transport_name } => {
            out.push(Fact::new(at("unimplemented"), transport_name));
        }
    }
}

/// One resolved event pattern.
///
/// Destructured for `reply_event`, which carries
/// `skip_serializing_if`: an RPC request whose reply the resolver could
/// not infer would otherwise print nothing, and a reviewer cannot act
/// on an absence.
///
/// The wire value is also given its name.
/// [`EventPatternInfo::pattern_kind_value`] is what codegen needs and
/// is a number on the page;
/// [`crate::mesh::pattern::CommunicationPattern::from_wire`] is its
/// declared inverse, so the name is the type's own answer and not a
/// second opinion about what a 2 means.
fn push_event_pattern(name: &str, p: &EventPatternInfo, out: &mut Vec<Fact>) {
    // ⚠ No `..`.
    let EventPatternInfo {
        event,
        pattern_kind_value,
        reply_event,
    } = p;
    out.push(Fact::new(format!("{name}.event"), event));
    let named = CommunicationPattern::from_wire(*pattern_kind_value)
        .map(|p| p.prefix_str().to_string())
        .unwrap_or_else(|| format!("(unnamed wire value {pattern_kind_value})"));
    out.push(Fact::new(format!("{name}.pattern"), named));
    match reply_event {
        Some(e) => out.push(Fact::new(format!("{name}.reply"), e)),
        None => out.push(Fact::new(format!("{name}.reply"), "(none)")),
    }
}

/// The authentication the deployment requires of this peer.
///
/// ⚠ No `..`. Two of its three fields carry `skip_serializing_if`, and
/// this is the one place in the whole surface where a vanished field is
/// a security statement: a `zenoh` binding with `required: true` and no
/// pinned fingerprint on the page reads exactly like one whose
/// fingerprint the reviewer simply was not shown.
///
/// ⚠⚠ This type was NOT found by reading the sources by hand. Three
/// hand measurements said `deploy.rs` carried no skips at all, each
/// wrong for its own reason, and
/// `nothing_the_builder_leaves_to_serde_can_skip_a_field` found it in
/// one run. The gate is the instrument; the reading was not.
fn push_auth(name: &str, a: &crate::mesh::deploy::AuthPolicyConfig, out: &mut Vec<Fact>) {
    let crate::mesh::deploy::AuthPolicyConfig {
        required,
        peer_fingerprint,
        sd_denied_classifies_as_unauthorized,
    } = a;
    out.push(Fact::new(format!("{name}.required"), required.to_string()));
    push_optional_scalar(
        &format!("{name}.peer-fingerprint"),
        peer_fingerprint.as_ref(),
        out,
    );
    push_optional_scalar(
        &format!("{name}.sd-denied-is-unauthorized"),
        sd_denied_classifies_as_unauthorized.as_ref(),
        out,
    );
}

/// A scalar the deployment may or may not have settled.
fn push_optional_scalar<T: std::fmt::Display>(name: &str, value: Option<&T>, out: &mut Vec<Fact>) {
    match value {
        Some(v) => out.push(Fact::new(name, v.to_string())),
        None => out.push(Fact::new(name, "(default)")),
    }
}

/// Every scalar of one nested value, under `name`.
fn push_nested<T: serde::Serialize>(name: &str, value: &T, out: &mut Vec<Fact>) {
    match serde_json::to_value(value) {
        Ok(v) => flatten(name, &v, out),
        // Unreachable for these owned types; said rather than skipped,
        // because a review surface that drops a field silently is the
        // failure this whole module exists to prevent.
        Err(e) => out.push(Fact::new(name, format!("(unreadable: {e})"))),
    }
}

/// A list of strings, or the fact that it is empty.
fn push_list(name: &str, items: &[String], out: &mut Vec<Fact>) {
    if items.is_empty() {
        out.push(Fact::new(name, "(none)"));
        return;
    }
    for (i, item) in items.iter().enumerate() {
        out.push(Fact::new(format!("{name}.{i}"), item));
    }
}

/// A list of nested values, or the fact that it is empty.
fn push_nested_list<T: serde::Serialize>(name: &str, items: &[T], out: &mut Vec<Fact>) {
    if items.is_empty() {
        out.push(Fact::new(name, "(none)"));
        return;
    }
    for (i, item) in items.iter().enumerate() {
        push_nested(&format!("{name}.{i}"), item, out);
    }
}

/// A value the deployment may or may not have settled.
///
/// The absent case is a fact, not a silence: `(none)` is what tells a
/// reviewer that the deployment was asked and said no.
fn push_optional<T: serde::Serialize>(name: &str, value: Option<&T>, out: &mut Vec<Fact>) {
    match value {
        Some(v) => push_nested(name, v, out),
        None => out.push(Fact::new(name, "(none)")),
    }
}

/// Every scalar in `value`, named by its path.
///
/// `retry.max_attempts`, `responders.0`. A path rather than a leaf name
/// because two nested structures can carry the same leaf, and a
/// reviewer reading `max_attempts` twice cannot tell which is which.
fn flatten(path: &str, value: &serde_json::Value, out: &mut Vec<Fact>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                flatten(&join(path, k), v, out);
            }
        }
        serde_json::Value::Array(items) => {
            // An empty array is written as a fact rather than skipped:
            // "no responders" and "the field is gone" are different
            // answers, and only one of them means the deployment said
            // something.
            if items.is_empty() {
                out.push(Fact::new(path, "(none)"));
            }
            for (i, v) in items.iter().enumerate() {
                flatten(&join(path, &i.to_string()), v, out);
            }
        }
        serde_json::Value::Null => out.push(Fact::new(path, "(none)")),
        serde_json::Value::String(s) => out.push(Fact::new(path, s)),
        other => out.push(Fact::new(path, other.to_string())),
    }
}

fn join(path: &str, part: &str) -> String {
    if path.is_empty() {
        part.to_string()
    } else {
        format!("{path}.{part}")
    }
}
