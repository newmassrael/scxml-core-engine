// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! How deep a Rust machine's internal event queue must be, proven from its
//! document (docs/adr/0076).
//!
//! The internal queue holds only what the machine raises for itself during a
//! macrostep, so the most it can hold follows from the document, where the
//! external queue's depth is the host's statement. A push past a bare-metal
//! queue's depth stops the program, so the number this module gives is a
//! claim that must hold for every run: an undercount stops a machine. Where
//! the claim cannot be made from what the model says, the answer is
//! [`InternalQueueBound::NotProven`] with the reason, and the queue takes the
//! external rule.
//!
//! # The bound
//!
//! Each block of executable content (an `<onentry>`, an `<onexit>`, a
//! transition's content, an `<initial>` transition's content, a `<history>`
//! default transition's content) is counted for the most internal events one
//! run of it can enqueue. `R`, the most one microstep can enqueue, is the sum
//! over the blocks (each runs at most once in a microstep, since the exit set,
//! the transition set and the entry set are sets), the `done.state` events and
//! the `<donedata>` errors entering states can raise, and what the start of a
//! macrostep can raise before its first microstep (see below; adding it to `R`
//! rather than once is an overcount, never an undercount). Then:
//!
//! - `R == 0`: nothing reaches the internal queue, and its depth is `0`;
//! - when no transition's event descriptor matches an event the internal queue
//!   can hold, a dequeued internal event takes no microstep; when, besides,
//!   every eventless transition has a target, reads no `_event` in its
//!   condition and no chain of them can return to itself, at most `1 + D`
//!   microsteps run before the first internal dequeue, `D` being the most
//!   eventless transitions one chain of microsteps can take, and the bound is
//!   `R x (1 + D)`;
//! - otherwise nothing is proven.
//!
//! Two eventless transitions follow one another when the source of the second
//! can be in the configuration with a target of the first: an ancestor or a
//! descendant of it, or a state in another region of a `<parallel>` above
//! both. That last case is why a `<parallel>` whose regions each take an
//! eventless transition is not proven: either region's can follow the other's,
//! since a condition the one's content makes true can enable the other's on
//! the next microstep.
//!
//! # Where an internal event comes from
//!
//! `Engine::raise` in `backends/rust/runtime/src/engine.rs` is the only code
//! that puts an event on the internal queue. Every caller is listed here with
//! the construct that causes it, the runtime profile that reaches it, and how
//! this analysis counts it. A new caller belongs in this list and in the walk
//! below, or in the refusals that make the bound not proven.
//!
//! The generated code (`tools/codegen/templates/rust`), each `engine.raise(`:
//!
//! - `actions/raise.rs.jinja2`: `<raise>`, its event; both profiles. One.
//! - `actions/send.rs.jinja2`: a `<send>` to `#_internal`, its event (under
//!   `no_std` a delayed one raises at once); a computed `targetexpr` handed to
//!   `Engine::send_to_target` (std), its event when the target is `#_internal`;
//!   `error.execution` for an argument that cannot be evaluated or a type,
//!   target or delay refused, and once more for each `<param>` that cannot be
//!   read, the message still going before the block ends; `error.communication`
//!   for a target nothing answers at (`#_parent` under `no_std`). Counted as its
//!   delivery (its event, or the error of a refusal or of a delivery nothing
//!   answers) with one for each fallible `<param>` and content before it, or
//!   one for an argument that stops it. A delayed send to anything
//!   but this session's external queue is not proven: it is delivered after
//!   the macrostep that made it. An internal send whose event name is computed
//!   is not proven: it can raise any event the document names.
//! - `actions/assign.rs.jinja2`, `actions/log.rs.jinja2`,
//!   `actions/sce_append.rs.jinja2` through `actions/_native_statement.rs.jinja2`:
//!   `error.execution` for a statement the static lowering marks
//!   `native_fails`, which ends the block; both profiles. One per block at
//!   most, since the first error ends it.
//! - `actions/cancel.rs.jinja2`: `error.execution` for a `native_sendid_fails`
//!   id, ending the block. Counted as a statement that fails.
//! - `actions/if.rs.jinja2`: `error.execution` for each condition that fails
//!   (`native_cond_fails`, raised by the lowered condition); the chain goes on
//!   to the next branch and the block ends after it. Counted as one for each
//!   such condition before the branch taken, with the branch.
//! - `actions/foreach.rs.jinja2`: the native loop raises nothing of its own;
//!   its body runs once per item, so a body that enqueues anything is not
//!   proven, and a body's error ends the block and is counted once.
//! - `actions/native_action.rs.jinja2` (the call `forge::native_action`
//!   renders): `error.execution` when an argument fails or the payload it reads
//!   was not delivered, and only when the model declares that event; the block
//!   goes on. One for each `<sce:action>` with an argument; one with none is a
//!   bare host call (`forge::native_action`, `CallRendering::lower`) and
//!   raises nothing.
//! - `entry_exit_actions.rs.jinja2`: `done.state.<parent>` on entering a
//!   `<final>` whose parent is a state, and `done.state.<grandparent>` when the
//!   `<parallel>` above it completes; `error.execution` for each `<donedata>`
//!   `<param>` and content that cannot be computed. Counted as one for each
//!   compound state with a `<final>` child, and one plus one for each `<final>`
//!   child for each `<parallel>`: a compound enters one child in a microstep,
//!   a `<parallel>` completes once in it, and each of its own `<final>`
//!   children raises for it.
//! - `process_transition.rs.jinja2`: a guard that fails (`native_guard_fails`)
//!   raises `error.execution` each time a selection evaluates it. A guard of an
//!   eventless transition is evaluated again after every internal dequeue, so
//!   one that can fail is not proven; a guard of a transition with an event is
//!   evaluated only for the event that starts a macrostep (no transition takes
//!   an internal event), at most once for each atomic state of the
//!   configuration, and is counted that many times.
//! - `datamodel_macros.rs.jinja2`, `scriptengine_helpers.rs.jinja2` and the
//!   script-engine arms of the statements above: std only. A document a script
//!   engine evaluates is not proven: any expression can fail, in its data
//!   bindings, its `<donedata>` and every guard.
//! - `invoke_methods.rs.jinja2`, `invoke_macros.rs.jinja2`,
//!   `param_macros.rs.jinja2`: std only. A document with an `<invoke>` is not
//!   proven: an invocation raises its errors when it starts, after the
//!   macrostep, and its `<finalize>` runs before the event it forwards.
//!
//! The engine's own callers, all std only:
//!
//! - `send_named_internal`: a `<send target="#_internal">` with a computed name
//!   (not proven, above); delayed, it waits for `deliver_routed`.
//! - `send_to_target`: a computed target that is `#_internal` (counted with the
//!   send).
//! - `deliver_routed`: a delayed routed send at the end of its delay, its event
//!   or `error.communication` (not proven, above).
//! - `perform_deferred_host_send`: a delayed host-served send, `error.execution`
//!   (not proven, above).
//! - `refuse_unserved_http_send`: a BasicHTTP send in a build without
//!   `http-send`, `error.execution` (counted with the send).
//! - `bind_lifted_payload`: at every dequeue, `error.execution` for a typed
//!   payload its data cannot be lifted to. At an internal dequeue it replaces
//!   the event it was raised for; at the external dequeue that starts a
//!   macrostep it is counted once, for a document that imports event schemas.
//! - `perform_host_invoke`, `start_host_invoke`, `start_restored_host_invokes`:
//!   a host-run `<invoke>` (not proven, above).
//!
//! `Engine::raise` is public: a host that calls it puts events on the
//! internal queue that no document accounts for, and is outside this bound.

use std::collections::{BTreeMap, BTreeSet};

use crate::event_descriptor::attribute_matches;
use crate::microstep_bounds::MicrostepBounds;
use crate::model::{Action, DoneData, DoneDataContent, SCXMLModel};

/// The error events generated code and the engine raise into the internal
/// queue, each only in a document that names it (a machine's event enum has
/// no member for an event the document does not).
const ERROR_EVENTS: [&str; 2] = ["error.execution", "error.communication"];

/// The internal queue's depth for one document, or why none is proven.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InternalQueueBound {
    /// The queue never holds more than `bound` events, for the reason `case`.
    Proven {
        /// The depth.
        bound: usize,
        /// Which case of the bound gave it.
        case: ProvenCase,
    },
    /// No depth is proven, and the queue takes the external queue's.
    NotProven {
        /// The construct that keeps the bound from being proven, in words the
        /// generated policy repeats.
        reason: String,
    },
}

/// Which case of the bound a proven depth comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenCase {
    /// Nothing in the document reaches the internal queue: the depth is `0`.
    NothingRaises,
    /// At most `per_microstep` events a microstep, over at most
    /// `1 + eventless_depth` microsteps before the first internal dequeue.
    Chain {
        /// `R`: the most one microstep can enqueue, with what the start of a
        /// macrostep can raise before its first microstep.
        per_microstep: usize,
        /// `D`: the most eventless transitions one chain of microsteps takes.
        eventless_depth: usize,
    },
}

impl InternalQueueBound {
    /// The bound of `model`, read as the static lowering left it: which
    /// statement and which guard can fail is known only once it has lowered
    /// them.
    pub fn of(model: &SCXMLModel) -> Self {
        match Analysis::new(model).run() {
            Ok(bound) => bound,
            Err(reason) => Self::NotProven { reason },
        }
    }

    /// The proven depth, `None` when none is.
    pub fn depth(&self) -> Option<usize> {
        match self {
            Self::Proven { bound, .. } => Some(*bound),
            Self::NotProven { .. } => None,
        }
    }

    /// The reason for the depth, as the generated policy states it, so a
    /// change to the document that moves the depth is visible in the
    /// generated diff.
    pub fn basis(&self) -> String {
        match self {
            Self::Proven {
                case: ProvenCase::NothingRaises,
                ..
            } => "Proven depth 0: nothing this document does can put an event on its internal \
                  queue."
                .to_string(),
            Self::Proven {
                bound,
                case:
                    ProvenCase::Chain {
                        per_microstep,
                        eventless_depth,
                    },
            } => format!(
                "Proven depth {bound}: one microstep enqueues at most {per_microstep} internal \
                 event(s), and at most 1 + {eventless_depth} microstep(s) run before the first \
                 internal event is taken, so {per_microstep} x (1 + {eventless_depth}) = {bound}."
            ),
            Self::NotProven { reason } => {
                format!("No depth is proven, so it is the external queue's: {reason}.")
            }
        }
    }

    /// [`basis`](Self::basis) in lines of a comment's width.
    pub fn basis_lines(&self) -> Vec<String> {
        wrap(&self.basis(), 72)
    }
}

/// What one statement, or one list of them, can put on the internal queue in
/// one run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Cost {
    /// At most, when it runs to its end and the list goes on.
    completes: usize,
    /// At most, when an error raised in it ends the block it is in; `0` when
    /// it cannot end one.
    ends: usize,
}

impl Cost {
    const NONE: Self = Self {
        completes: 0,
        ends: 0,
    };

    fn completing(events: usize) -> Self {
        Self {
            completes: events,
            ends: 0,
        }
    }

    fn ending(events: usize) -> Self {
        Self {
            completes: 0,
            ends: events,
        }
    }

    /// The most a run of it enqueues, however it ends.
    fn worst(self) -> usize {
        self.completes.max(self.ends)
    }
}

/// One walk of a document, collecting what the bound needs.
struct Analysis<'m> {
    model: &'m SCXMLModel,
    /// Whether generated code can raise `error.execution`: every site that
    /// does is written under the document's naming it.
    raises_execution: bool,
    /// Whether it can raise `error.communication`, likewise.
    raises_communication: bool,
    /// The events the internal queue can hold, by name, errors aside.
    held: BTreeSet<String>,
    /// Whether some counted site raises an error event.
    may_raise_error: bool,
}

impl<'m> Analysis<'m> {
    fn new(model: &'m SCXMLModel) -> Self {
        Self {
            model,
            raises_execution: model.events.contains("error.execution"),
            raises_communication: model.events.contains("error.communication"),
            held: BTreeSet::new(),
            may_raise_error: false,
        }
    }

    fn run(mut self) -> Result<InternalQueueBound, String> {
        let model = self.model;
        if model.needs_script_engine {
            return Err(
                "a script engine evaluates this document's expressions, and one that \
                        fails raises error.execution in places this bound does not count (data \
                        bindings, <donedata> and the guards of every selection)"
                    .to_string(),
            );
        }
        if let Some((id, _)) = model.states.iter().find(|(_, s)| !s.invokes.is_empty()) {
            return Err(format!(
                "the state `{id}` holds an <invoke>, which raises its errors when it starts, \
                 after the macrostep, and whose <finalize> runs before the event it forwards"
            ));
        }
        if self.raises_execution {
            for (id, state) in &model.states {
                if state
                    .transitions
                    .iter()
                    .any(|t| t.event.is_empty() && t.native_guard_fails)
                {
                    return Err(format!(
                        "the guard of an eventless transition of `{id}` can fail, and the \
                         eventless selection that raises error.execution for it runs again \
                         after every internal event"
                    ));
                }
            }
        }

        let mut per_microstep = 0usize;
        for state in model.states.values() {
            for block in state.on_entry_blocks.iter().chain(&state.on_exit_blocks) {
                per_microstep = per_microstep.saturating_add(self.block(block)?);
            }
            for transition in &state.transitions {
                per_microstep = per_microstep.saturating_add(self.block(&transition.actions)?);
            }
            per_microstep =
                per_microstep.saturating_add(self.block(&state.initial_transition_actions)?);
            if state.is_final {
                if let Some(data) = &state.donedata {
                    per_microstep = per_microstep.saturating_add(self.donedata(data));
                }
            }
        }
        for history in model.history_states.values() {
            per_microstep = per_microstep.saturating_add(self.block(&history.default_actions)?);
        }
        per_microstep = per_microstep.saturating_add(self.done_events());
        per_microstep = per_microstep.saturating_add(self.selection_errors());
        per_microstep = per_microstep.saturating_add(self.payload_lift());

        if per_microstep == 0 {
            return Ok(InternalQueueBound::Proven {
                bound: 0,
                case: ProvenCase::NothingRaises,
            });
        }
        self.no_transition_takes_an_internal_event()?;
        let eventless_depth = self.eventless_depth()?;
        let bound = per_microstep
            .checked_mul(eventless_depth.saturating_add(1))
            .filter(|bound| *bound <= usize::from(u16::MAX))
            .ok_or_else(|| {
                format!(
                    "{per_microstep} x (1 + {eventless_depth}) is more than a queue counts \
                     (65535)"
                )
            })?;
        Ok(InternalQueueBound::Proven {
            bound,
            case: ProvenCase::Chain {
                per_microstep,
                eventless_depth,
            },
        })
    }

    /// One `error.execution`, when the document names that event.
    fn execution_error(&mut self) -> usize {
        if self.raises_execution {
            self.may_raise_error = true;
            1
        } else {
            0
        }
    }

    /// One error of a `<send>`, `error.execution` or `error.communication`,
    /// when the document names either.
    fn send_error(&mut self) -> usize {
        if self.raises_execution || self.raises_communication {
            self.may_raise_error = true;
            1
        } else {
            0
        }
    }

    /// The most one run of a block enqueues.
    fn block(&mut self, actions: &[Action]) -> Result<usize, String> {
        Ok(self.list(actions)?.worst())
    }

    /// A list of statements: what each runs to its end enqueues, or, when one
    /// ends the block, what those before it enqueued and what it did.
    fn list(&mut self, actions: &[Action]) -> Result<Cost, String> {
        let mut cost = Cost::NONE;
        for action in actions {
            let one = self.statement(action)?;
            cost.ends = cost.ends.max(cost.completes.saturating_add(one.ends));
            cost.completes = cost.completes.saturating_add(one.completes);
        }
        Ok(cost)
    }

    fn statement(&mut self, action: &Action) -> Result<Cost, String> {
        match action.action_type.as_str() {
            "raise" => {
                self.held.insert(action.event.clone());
                Ok(Cost::completing(1))
            }
            "send" => self.send(action),
            "assign" | "log" | "sce_append" => {
                if !action.native_code.is_empty() && action.native_fails {
                    Ok(Cost::ending(self.execution_error()))
                } else {
                    Ok(Cost::NONE)
                }
            }
            "cancel" => {
                if action.native_sendid_fails {
                    Ok(Cost::ending(self.execution_error()))
                } else {
                    Ok(Cost::NONE)
                }
            }
            "sce_clear" => Ok(Cost::NONE),
            // A native action with no argument is a bare host call, which
            // cannot fail; one with arguments raises when one fails or the
            // payload it reads was not delivered.
            "native_action" if action.params.is_empty() => Ok(Cost::NONE),
            "native_action" => Ok(Cost::completing(self.execution_error())),
            "if" => self.conditional(action),
            "foreach" => self.foreach(action),
            "script" if action.content.is_empty() => Ok(Cost::NONE),
            other => Err(format!(
                "a <{other}> statement is not one this bound knows how to count"
            )),
        }
    }

    /// An `<if>`: the branch the chain takes, after the conditions before it,
    /// each of which may have failed and raised its error. A failed condition
    /// is false, so the chain goes on, and ends the block once it has run.
    fn conditional(&mut self, action: &Action) -> Result<Cost, String> {
        let mut branches: Vec<(bool, &[Action])> =
            vec![(action.native_cond_fails, action.then_actions.as_slice())];
        branches.extend(
            action
                .elseif_branches
                .iter()
                .map(|branch| (branch.native_cond_fails, branch.actions.as_slice())),
        );
        let mut cost = Cost::NONE;
        let mut failed_before = 0usize;
        for (condition_fails, branch) in branches {
            let run = self.list(branch)?;
            cost.completes = cost.completes.max(run.completes);
            cost.ends = cost.ends.max(after_conditions(failed_before, run));
            if condition_fails {
                failed_before = failed_before.saturating_add(self.execution_error());
            }
        }
        let otherwise = self.list(&action.else_actions)?;
        cost.completes = cost.completes.max(otherwise.completes);
        cost.ends = cost.ends.max(after_conditions(failed_before, otherwise));
        Ok(cost)
    }

    /// A `<foreach>`: its body runs once per item, and the items are not
    /// counted, so a body that enqueues anything is not proven. A body's error
    /// ends the block, so it is raised once.
    fn foreach(&mut self, action: &Action) -> Result<Cost, String> {
        let body = self.list(&action.actions)?;
        if body.completes > 0 {
            return Err(
                "a <foreach> body enqueues an internal event on each pass, and the passes are \
                 not counted"
                    .to_string(),
            );
        }
        Ok(Cost::ending(body.ends))
    }

    /// A `<send>`: its delivery, and its errors when it can fail.
    fn send(&mut self, action: &Action) -> Result<Cost, String> {
        let host_served = action.send_type_host_served;
        let to_internal =
            !host_served && action.targetexpr.is_empty() && action.target == "#_internal";
        let to_own_external = !host_served
            && action.target.is_empty()
            && crate::scxml_self_send::sends_to_itself(action);
        let computed_target = !action.targetexpr.is_empty() || !action.native_target.is_empty();
        let computed_event = !action.eventexpr.is_empty() || !action.native_event.is_empty();
        let delayed = !action.native_delay.is_empty()
            || !action.delayexpr.is_empty()
            || action.delay_ms > 0
            || (!action.delay.is_empty() && action.delay != "0s" && action.delay != "0ms");
        let refused = action.send_type_unsupported
            || action.target_unsupported
            || (action.delay_invalid && action.delayexpr.is_empty());

        let failing_params = action
            .params
            .iter()
            .filter(|param| {
                !param.is_static_literal && (param.native_value.is_empty() || param.native_fails)
            })
            .count();
        let failing_values = if self.raises_execution {
            failing_params + usize::from(action.native_content_value_fails)
        } else {
            0
        };
        if failing_values > 0 {
            self.may_raise_error = true;
        }
        // A send to this session's own queue, with nothing computed and
        // nothing that can fail, raises no error; any other can.
        let can_fail = refused
            || failing_values > 0
            || !(to_internal || to_own_external)
            || computed_event
            || computed_target
            || !action.native_delay.is_empty();
        let error = if can_fail { self.send_error() } else { 0 };

        let delivered = if refused {
            0
        } else if delayed {
            if !to_own_external {
                let target = if computed_target {
                    "a computed target"
                } else if action.target.is_empty() {
                    "another processor"
                } else {
                    action.target.as_str()
                };
                return Err(format!(
                    "a delayed <send> to {target} is delivered after the macrostep that made it, \
                     and its delivery can reach the internal queue"
                ));
            }
            0
        } else if to_internal || computed_target {
            if computed_event {
                return Err(
                    "a <send> that can reach #_internal computes its event's name, so it \
                            can put any event the document names on the internal queue"
                        .to_string(),
                );
            }
            self.held.insert(action.event.clone());
            1
        } else if to_own_external {
            0
        } else {
            // A literal target that is not this session, or another
            // processor: what reaches the internal queue is the error of a
            // delivery nothing answers.
            error
        };

        // What the send raises once its payload is built: a refusal, or a
        // delivery nothing answers. A send to this session's own queue is
        // delivered whenever it is not refused.
        let after_payload = if refused || !(to_internal || to_own_external) {
            error
        } else {
            0
        };
        // An argument that cannot be evaluated raises one error and sends
        // nothing; otherwise each failed value raises its own, the message
        // goes, and the block ends.
        let ends = if can_fail {
            error.max(failing_values.saturating_add(delivered.max(after_payload)))
        } else {
            0
        };
        Ok(Cost {
            completes: if refused { 0 } else { delivered },
            ends,
        })
    }

    /// The `<donedata>` of a `<final>`: one `error.execution` for each
    /// `<param>` and content that cannot be computed, raised as it is entered.
    fn donedata(&mut self, data: &DoneData) -> usize {
        let params = data
            .params
            .iter()
            .filter(|param| param.native_value.is_empty() || param.native_fails)
            .count();
        let content = if !data.native_content.is_empty() {
            0
        } else if !data.native_content_value.is_empty() {
            usize::from(data.native_content_value_fails)
        } else {
            usize::from(matches!(
                data.content,
                DoneDataContent::Expression(_) | DoneDataContent::InlineText(_)
            ))
        };
        let failing = params + content;
        if failing > 0 && self.raises_execution {
            self.may_raise_error = true;
            failing
        } else {
            0
        }
    }

    /// The `done.state` events one microstep can raise: one for each compound
    /// state with a `<final>` child, which enters one child at a time, and for
    /// each `<parallel>` one for its completing plus one for each `<final>`
    /// child it has, each of which raises for it on entry.
    fn done_events(&mut self) -> usize {
        let model = self.model;
        let mut final_children: BTreeMap<&str, usize> = BTreeMap::new();
        for state in model.states.values() {
            if let (true, Some(parent)) = (state.is_final, state.parent.as_deref()) {
                *final_children.entry(parent).or_default() += 1;
            }
        }
        let mut count = 0usize;
        for (id, state) in &model.states {
            let finals = final_children.get(id.as_str()).copied().unwrap_or(0);
            let raised = if state.is_parallel {
                1 + finals
            } else {
                usize::from(finals > 0)
            };
            if raised > 0 {
                self.held.insert(format!("done.state.{id}"));
                count = count.saturating_add(raised);
            }
        }
        count
    }

    /// The errors of the selection for the event that starts a macrostep: a
    /// guard of a transition with an event that can fail is evaluated at most
    /// once for each atomic state of the configuration.
    fn selection_errors(&mut self) -> usize {
        if !self.raises_execution {
            return 0;
        }
        let fallible = self
            .model
            .states
            .values()
            .flat_map(|state| &state.transitions)
            .filter(|t| !t.event.is_empty() && t.native_guard_fails)
            .count();
        if fallible == 0 {
            return 0;
        }
        self.may_raise_error = true;
        fallible.saturating_mul(MicrostepBounds::of(self.model).enabled)
    }

    /// The error the engine raises when the event that starts a macrostep
    /// carries data its typed payload cannot be lifted from (std only).
    fn payload_lift(&mut self) -> usize {
        if self.raises_execution && !self.model.imported_event_schemas.is_empty() {
            self.may_raise_error = true;
            1
        } else {
            0
        }
    }

    /// No transition takes an event the internal queue can hold, so a dequeued
    /// internal event starts no microstep.
    fn no_transition_takes_an_internal_event(&self) -> Result<(), String> {
        let model = self.model;
        let errors = ERROR_EVENTS
            .iter()
            .filter(|name| self.may_raise_error && model.events.contains(**name))
            .map(|name| (*name).to_string());
        let held: Vec<String> = self.held.iter().cloned().chain(errors).collect();
        for (id, state) in &model.states {
            for transition in &state.transitions {
                if transition.event.is_empty() {
                    continue;
                }
                if let Some(name) = held.iter().find(|name| {
                    transition.matches_any_event || attribute_matches(&transition.event, name)
                }) {
                    return Err(format!(
                        "the transition of `{id}` on `{}` takes `{name}`, an event the internal \
                         queue can hold, so an internal event can start a microstep that \
                         raises more",
                        transition.event
                    ));
                }
            }
        }
        Ok(())
    }

    /// `D`: the most eventless transitions one chain of microsteps can take.
    fn eventless_depth(&self) -> Result<usize, String> {
        let model = self.model;
        // Each eventless transition as its source and the states its targets
        // enter: a `<history>` enters its parent and states below it.
        let mut eventless: Vec<(&str, Vec<&str>)> = Vec::new();
        for (id, state) in &model.states {
            for transition in state.transitions.iter().filter(|t| t.event.is_empty()) {
                if transition.cond.contains("_event") {
                    return Err(format!(
                        "the condition of an eventless transition of `{id}` reads `_event`, \
                         which each internal event changes, so an eventless microstep can \
                         follow any of them"
                    ));
                }
                if transition.targets.is_empty() {
                    return Err(format!(
                        "an eventless transition of `{id}` has no target, so it can be taken \
                         again on the next microstep"
                    ));
                }
                let entered = transition
                    .targets
                    .iter()
                    .map(|target| match model.history_states.get(target) {
                        Some(history) => history.parent.as_str(),
                        None => target.as_str(),
                    })
                    .collect();
                eventless.push((id.as_str(), entered));
            }
        }
        let tree = Tree { model };
        let next: Vec<Vec<usize>> = eventless
            .iter()
            .map(|(_, entered)| {
                (0..eventless.len())
                    .filter(|&j| entered.iter().any(|t| tree.coexist(t, eventless[j].0)))
                    .collect()
            })
            .collect();
        let mut longest: Vec<Option<usize>> = vec![None; eventless.len()];
        let mut on_path = vec![false; eventless.len()];
        let mut depth = 0usize;
        for start in 0..eventless.len() {
            let from = longest_chain(start, &next, &mut longest, &mut on_path).map_err(|at| {
                format!(
                    "the eventless transitions of `{}` can follow one another in a cycle, so \
                     the microsteps of one macrostep are not bounded",
                    eventless[at].0
                )
            })?;
            depth = depth.max(from);
        }
        Ok(depth)
    }
}

/// What a branch of an `<if>` taken after `failed` errors of its conditions
/// enqueues when the `<if>` ends its block: the errors and the whole branch,
/// since the block ends after the chain whatever the branch does, or, when no
/// condition failed, what the branch enqueues when it ends the block itself.
fn after_conditions(failed: usize, run: Cost) -> usize {
    if failed > 0 {
        failed.saturating_add(run.worst())
    } else {
        run.ends
    }
}

/// The most transitions on a chain from `start` through `next`, or a
/// transition on a cycle when the chain can return to one it has taken.
fn longest_chain(
    start: usize,
    next: &[Vec<usize>],
    longest: &mut [Option<usize>],
    on_path: &mut [bool],
) -> Result<usize, usize> {
    if let Some(known) = longest[start] {
        return Ok(known);
    }
    if on_path[start] {
        return Err(start);
    }
    on_path[start] = true;
    let mut below = 0usize;
    for &following in &next[start] {
        below = below.max(longest_chain(following, next, longest, on_path)?);
    }
    on_path[start] = false;
    longest[start] = Some(below + 1);
    Ok(below + 1)
}

/// The states of a document, as which can be in one configuration together.
struct Tree<'m> {
    model: &'m SCXMLModel,
}

impl Tree<'_> {
    /// `id` and every state above it, nearest first. A parent chain that loops
    /// is cut where it repeats, as the parser refuses one elsewhere.
    fn up_from<'s>(&'s self, id: &'s str) -> Vec<&'s str> {
        let mut chain = vec![id];
        let mut current = id;
        while let Some(parent) = self
            .model
            .states
            .get(current)
            .and_then(|state| state.parent.as_deref())
        {
            if chain.contains(&parent) {
                break;
            }
            chain.push(parent);
            current = parent;
        }
        chain
    }

    /// Whether `a` and `b` can be in one configuration: one is the other or
    /// above it, or the nearest state above both is a `<parallel>`, whose
    /// regions are all active at once.
    fn coexist(&self, a: &str, b: &str) -> bool {
        let above_a = self.up_from(a);
        if above_a.contains(&b) {
            return true;
        }
        let above_b = self.up_from(b);
        if above_b.contains(&a) {
            return true;
        }
        above_a
            .iter()
            .find(|state| above_b.contains(*state))
            .and_then(|common| self.model.states.get(*common))
            .is_some_and(|common| common.is_parallel)
    }
}

/// `text` in lines of at most `width` characters, broken between words.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ElseIfBranch, Param, State, Transition};

    /// A document built state by state, as the static lowering leaves one.
    #[derive(Default)]
    struct Doc {
        model: SCXMLModel,
    }

    impl Doc {
        fn new() -> Self {
            Self::default()
        }

        fn add(
            mut self,
            id: &str,
            parent: Option<&str>,
            is_final: bool,
            is_parallel: bool,
        ) -> Self {
            self.model.states.insert(
                id.to_string(),
                State {
                    id: id.to_string(),
                    parent: parent.map(str::to_string),
                    is_final,
                    is_parallel,
                    ..State::default()
                },
            );
            self
        }

        fn state(self, id: &str, parent: Option<&str>) -> Self {
            self.add(id, parent, false, false)
        }

        fn final_state(self, id: &str, parent: Option<&str>) -> Self {
            self.add(id, parent, true, false)
        }

        fn parallel(self, id: &str, parent: Option<&str>) -> Self {
            self.add(id, parent, false, true)
        }

        fn entry(mut self, id: &str, block: Vec<Action>) -> Self {
            self.model
                .states
                .get_mut(id)
                .expect("the state is added first")
                .on_entry_blocks
                .push(block);
            self
        }

        fn transition(mut self, id: &str, transition: Transition) -> Self {
            self.model
                .states
                .get_mut(id)
                .expect("the state is added first")
                .transitions
                .push(transition);
            self
        }

        /// The document names `event`, as the analyzer records an event a
        /// raise site of generated code needs a member for.
        fn names(mut self, event: &str) -> Self {
            self.model.events.insert(event.to_string());
            self
        }

        fn bound(&self) -> InternalQueueBound {
            InternalQueueBound::of(&self.model)
        }
    }

    fn raise(event: &str) -> Action {
        Action {
            action_type: "raise".to_string(),
            event: event.to_string(),
            ..Action::default()
        }
    }

    /// An `<assign>` the static lowering marked as one that can fail.
    fn failing_assign() -> Action {
        Action {
            action_type: "assign".to_string(),
            location: "x".to_string(),
            native_code: "self.x.checked_add(1)".to_string(),
            native_fails: true,
            ..Action::default()
        }
    }

    /// An `<sce:action>` with one argument, read from its event's payload.
    fn native_action(name: &str) -> Action {
        Action {
            action_type: "native_action".to_string(),
            native_action_name: name.to_string(),
            params: vec![Param {
                name: "n".to_string(),
                expr: "_event.data.n".to_string(),
                ..Param::default()
            }],
            ..Action::default()
        }
    }

    /// An `<sce:action>` with no argument: a bare host call.
    fn bare_native_action(name: &str) -> Action {
        Action {
            action_type: "native_action".to_string(),
            native_action_name: name.to_string(),
            ..Action::default()
        }
    }

    fn on(event: &str, target: &str, actions: Vec<Action>) -> Transition {
        Transition {
            event: event.to_string(),
            target: target.to_string(),
            targets: if target.is_empty() {
                Vec::new()
            } else {
                vec![target.to_string()]
            },
            actions,
            ..Transition::default()
        }
    }

    fn eventless(target: &str) -> Transition {
        on("", target, Vec::new())
    }

    fn chain(per_microstep: usize, eventless_depth: usize) -> InternalQueueBound {
        InternalQueueBound::Proven {
            bound: per_microstep * (1 + eventless_depth),
            case: ProvenCase::Chain {
                per_microstep,
                eventless_depth,
            },
        }
    }

    fn not_proven_because(bound: &InternalQueueBound, words: &str) {
        match bound {
            InternalQueueBound::NotProven { reason } => assert!(
                reason.contains(words),
                "the reason should name `{words}`: {reason}"
            ),
            proven => panic!("expected no proven depth ({words}), got {proven:?}"),
        }
    }

    /// Two states and two events, nothing raised: the internal queue has no
    /// slot, and the policy says so.
    #[test]
    fn a_document_that_raises_nothing_has_a_queue_of_depth_zero() {
        let doc = Doc::new()
            .state("a", None)
            .state("b", None)
            .transition("a", on("go", "b", Vec::new()))
            .transition("b", on("stop", "a", Vec::new()));
        let bound = doc.bound();
        assert_eq!(
            bound,
            InternalQueueBound::Proven {
                bound: 0,
                case: ProvenCase::NothingRaises
            }
        );
        assert_eq!(bound.depth(), Some(0));
        assert!(bound.basis().starts_with("Proven depth 0:"));
    }

    /// One `<raise>` in one block and no eventless transition: one event a
    /// microstep, one microstep before the first internal dequeue.
    #[test]
    fn a_raise_in_one_block_with_no_eventless_transition_is_a_depth_of_one() {
        let doc = Doc::new()
            .state("a", None)
            .state("b", None)
            .transition("a", on("go", "b", vec![raise("note")]));
        assert_eq!(doc.bound(), chain(1, 0));
        assert_eq!(doc.bound().depth(), Some(1));
    }

    /// Two eventless transitions in a row: the event's microstep and the two
    /// eventless ones, each of which can enter `b` or `c` and run what it
    /// raises.
    #[test]
    fn an_eventless_chain_of_two_is_three_microsteps() {
        let doc = Doc::new()
            .state("a", None)
            .state("b", None)
            .state("c", None)
            .state("d", None)
            .entry("b", vec![raise("note")])
            .transition("a", on("go", "b", Vec::new()))
            .transition("b", eventless("c"))
            .transition("c", eventless("d"));
        assert_eq!(doc.bound(), chain(1, 2));
        assert_eq!(doc.bound().depth(), Some(3));
    }

    /// An eventless transition whose source is an ancestor of the state the
    /// one before it entered follows it: the compound is still active.
    #[test]
    fn an_eventless_transition_of_an_ancestor_follows_one_into_its_child() {
        let doc = Doc::new()
            .state("a", None)
            .state("c", None)
            .state("c1", Some("c"))
            .state("c2", Some("c"))
            .state("out", None)
            .entry("c2", vec![raise("note")])
            .transition("a", on("go", "c1", Vec::new()))
            .transition("c1", eventless("c2"))
            .transition("c", eventless("out"));
        assert_eq!(doc.bound(), chain(1, 2));
    }

    #[test]
    fn an_eventless_cycle_is_not_proven() {
        let doc = Doc::new()
            .state("a", None)
            .state("b", None)
            .state("c", None)
            .entry("b", vec![raise("note")])
            .transition("a", on("go", "b", Vec::new()))
            .transition("b", eventless("c"))
            .transition("c", eventless("b"));
        not_proven_because(&doc.bound(), "cycle");
    }

    /// Two regions of a `<parallel>`, each with an eventless transition: the
    /// second can be enabled by what the first's microstep did, and the first
    /// by the second's, so no chain length is proven.
    #[test]
    fn eventless_transitions_in_two_regions_can_follow_each_other() {
        let doc = Doc::new()
            .state("idle", None)
            .parallel("p", None)
            .state("ra", Some("p"))
            .state("a1", Some("ra"))
            .state("a2", Some("ra"))
            .state("rb", Some("p"))
            .state("b1", Some("rb"))
            .state("b2", Some("rb"))
            .entry("a2", vec![raise("note")])
            .transition("idle", on("go", "p", Vec::new()))
            .transition("a1", eventless("a2"))
            .transition("b1", eventless("b2"));
        not_proven_because(&doc.bound(), "cycle");
    }

    /// Each internal event taken changes `_event`, so an eventless condition
    /// that reads it can enable a microstep after any of them.
    #[test]
    fn an_eventless_condition_that_reads_the_event_is_not_proven() {
        let mut reads_event = eventless("b");
        reads_event.cond = "_event.name == 'note'".to_string();
        let doc = Doc::new()
            .state("a", None)
            .state("b", None)
            .entry("a", vec![raise("note")])
            .transition("a", reads_event);
        not_proven_because(&doc.bound(), "_event");
    }

    #[test]
    fn a_targetless_eventless_transition_is_not_proven() {
        let doc = Doc::new()
            .state("a", None)
            .entry("a", vec![raise("note")])
            .transition("a", eventless(""));
        not_proven_because(&doc.bound(), "no target");
    }

    /// A transition on the error a statement raises takes it off the internal
    /// queue and starts a microstep, which can raise again.
    #[test]
    fn a_transition_on_error_execution_is_not_proven() {
        let doc = Doc::new()
            .names("error.execution")
            .state("a", None)
            .state("b", None)
            .entry("a", vec![failing_assign()])
            .transition("a", on("error.execution", "b", Vec::new()));
        not_proven_because(&doc.bound(), "error.execution");

        let wildcard = Doc::new()
            .names("error.execution")
            .state("a", None)
            .state("b", None)
            .entry("a", vec![failing_assign()])
            .transition("a", on("error.*", "b", Vec::new()));
        not_proven_because(&wildcard.bound(), "error.execution");
    }

    /// The same transition in a document where nothing can fail takes no
    /// internal event: the error it names is never raised.
    #[test]
    fn a_transition_on_an_error_nothing_raises_is_proven() {
        let doc = Doc::new()
            .names("error.execution")
            .state("a", None)
            .state("b", None)
            .entry("a", vec![raise("note")])
            .transition("a", on("error.execution", "b", Vec::new()));
        assert_eq!(doc.bound(), chain(1, 0));
    }

    #[test]
    fn a_transition_on_a_raised_event_is_not_proven() {
        let doc = Doc::new()
            .state("a", None)
            .state("b", None)
            .entry("a", vec![raise("go")])
            .transition("a", on("go", "b", Vec::new()));
        not_proven_because(&doc.bound(), "`go`");
    }

    #[test]
    fn a_foreach_whose_body_raises_is_not_proven() {
        let foreach = Action {
            action_type: "foreach".to_string(),
            native_loop: "for item in self.items.clone()".to_string(),
            actions: vec![raise("tick")],
            ..Action::default()
        };
        let doc = Doc::new().state("a", None).entry("a", vec![foreach]);
        not_proven_because(&doc.bound(), "<foreach>");
    }

    /// A `<foreach>` whose body only fails ends its block at the first error,
    /// so it raises one however many items it walks.
    #[test]
    fn a_foreach_whose_body_only_fails_raises_one_error() {
        let foreach = Action {
            action_type: "foreach".to_string(),
            native_loop: "for item in self.items.clone()".to_string(),
            actions: vec![failing_assign(), failing_assign()],
            ..Action::default()
        };
        let doc = Doc::new()
            .names("error.execution")
            .state("a", None)
            .entry("a", vec![foreach]);
        assert_eq!(doc.bound(), chain(1, 0));
    }

    /// A compound state with a `<final>` child raises `done.state.<id>` once
    /// in a microstep, whichever child it enters.
    #[test]
    fn a_compound_with_a_final_child_counts_its_done_event() {
        let doc = Doc::new()
            .state("c", None)
            .state("s", Some("c"))
            .final_state("f", Some("c"))
            .transition("s", on("go", "f", Vec::new()));
        assert_eq!(doc.bound(), chain(1, 0));

        let taken = Doc::new()
            .state("c", None)
            .state("s", Some("c"))
            .final_state("f", Some("c"))
            .state("after", None)
            .transition("s", on("go", "f", Vec::new()))
            .transition("c", on("done.state.c", "after", Vec::new()));
        not_proven_because(&taken.bound(), "done.state.c");
    }

    /// A `<parallel>` completes once in a microstep; each region that is a
    /// compound with a `<final>` raises its own done event besides.
    #[test]
    fn a_parallel_counts_its_completion_and_each_region_its_own() {
        let doc = Doc::new()
            .parallel("p", None)
            .state("r1", Some("p"))
            .state("r1a", Some("r1"))
            .final_state("r1f", Some("r1"))
            .state("r2", Some("p"))
            .transition("r1a", on("go", "r1f", Vec::new()));
        assert_eq!(doc.bound(), chain(2, 0));
    }

    /// The first error ends the block, so three statements that can fail in
    /// one block raise one error, and the same three in three blocks three.
    #[test]
    fn a_fallible_statement_is_counted_once_per_block() {
        let one_block = Doc::new().names("error.execution").state("a", None).entry(
            "a",
            vec![failing_assign(), failing_assign(), failing_assign()],
        );
        assert_eq!(one_block.bound(), chain(1, 0));

        let three_blocks = Doc::new()
            .names("error.execution")
            .state("a", None)
            .entry("a", vec![failing_assign()])
            .entry("a", vec![failing_assign()])
            .entry("a", vec![failing_assign()]);
        assert_eq!(three_blocks.bound(), chain(3, 0));
    }

    /// A statement that can fail raises nothing in a document that does not
    /// name `error.execution`: every raise site is written under it.
    #[test]
    fn a_fallible_statement_raises_nothing_without_the_error_event() {
        let doc = Doc::new()
            .state("a", None)
            .entry("a", vec![failing_assign(), failing_assign()]);
        assert_eq!(doc.bound().depth(), Some(0));
    }

    /// A `<cancel>` whose id cannot be computed raises its error and ends the
    /// block, as a failing statement does.
    #[test]
    fn a_cancel_whose_id_fails_counts_its_error() {
        let cancel = Action {
            action_type: "cancel".to_string(),
            native_sendid: "self.id()".to_string(),
            native_sendid_fails: true,
            ..Action::default()
        };
        let doc = Doc::new()
            .names("error.execution")
            .state("a", None)
            .entry("a", vec![cancel]);
        assert_eq!(doc.bound(), chain(1, 0));
    }

    /// A native action's error does not end its block, so each one counts.
    #[test]
    fn each_native_action_counts_its_error() {
        let doc = Doc::new()
            .names("error.execution")
            .state("a", None)
            .entry("a", vec![native_action("open"), native_action("close")]);
        assert_eq!(doc.bound(), chain(2, 0));

        let without_the_event = Doc::new()
            .state("a", None)
            .entry("a", vec![native_action("open"), native_action("close")]);
        assert_eq!(without_the_event.bound().depth(), Some(0));
    }

    /// A native action with no argument cannot fail, so a document of them
    /// raises nothing even where it names `error.execution`.
    #[test]
    fn a_native_action_with_no_argument_raises_nothing() {
        let doc = Doc::new()
            .names("error.execution")
            .state("a", None)
            .state("b", None)
            .entry("a", vec![bare_native_action("open")])
            .transition("a", on("go", "b", vec![bare_native_action("close")]));
        assert_eq!(doc.bound().depth(), Some(0));
    }

    /// Each condition of an `<if>` that fails raises its error and the chain
    /// goes on, so the `<else>` can follow two errors with its own event.
    #[test]
    fn each_failing_condition_of_an_if_counts_before_the_branch_taken() {
        let conditional = Action {
            action_type: "if".to_string(),
            native_cond_fails: true,
            then_actions: Vec::new(),
            elseif_branches: vec![ElseIfBranch {
                native_cond_fails: true,
                ..ElseIfBranch::default()
            }],
            else_actions: vec![raise("note")],
            ..Action::default()
        };
        let doc = Doc::new()
            .names("error.execution")
            .state("a", None)
            .entry("a", vec![conditional]);
        assert_eq!(doc.bound(), chain(3, 0));
    }

    /// A `<send>` goes with the pairs it could read, each failed `<param>`
    /// having raised its error, and then the block ends.
    #[test]
    fn each_failing_param_of_a_send_counts() {
        let failing_param = Param {
            name: "n".to_string(),
            native_value: "self.n.checked_add(1)".to_string(),
            native_fails: true,
            ..Param::default()
        };
        let send = Action {
            action_type: "send".to_string(),
            event: "report".to_string(),
            params: vec![failing_param.clone(), failing_param],
            ..Action::default()
        };
        let doc = Doc::new()
            .names("error.execution")
            .state("a", None)
            .entry("a", vec![send]);
        // Two param errors: it goes to this session's external queue, which
        // nothing refuses, so its delivery adds nothing here.
        assert_eq!(doc.bound(), chain(2, 0));
    }

    /// Entering a `<final>` computes its `<donedata>`, and each `<param>` that
    /// cannot be computed raises its error.
    #[test]
    fn each_failing_donedata_param_counts() {
        let failing = crate::model::DoneDataParam {
            name: "n".to_string(),
            native_value: "self.n.checked_add(1)".to_string(),
            native_fails: true,
            ..crate::model::DoneDataParam::default()
        };
        let mut doc = Doc::new()
            .names("error.execution")
            .state("a", None)
            .final_state("done", None)
            .transition("a", on("go", "done", Vec::new()));
        doc.model
            .states
            .get_mut("done")
            .expect("added above")
            .donedata = Some(DoneData {
            params: vec![failing.clone(), failing],
            ..DoneData::default()
        });
        assert_eq!(doc.bound(), chain(2, 0));
    }

    /// A guard of a transition with an event that can fail raises its error
    /// in the selection for the event that starts a macrostep, once for each
    /// atomic state that asks it.
    #[test]
    fn a_fallible_guard_counts_once_for_each_atomic_state_that_asks_it() {
        let mut guarded = on("go", "b", Vec::new());
        guarded.native_guard_fails = true;
        let flat = Doc::new()
            .names("error.execution")
            .state("a", None)
            .state("b", None)
            .transition("a", guarded.clone());
        assert_eq!(flat.bound(), chain(1, 0));

        // On a `<parallel>` of two atomic regions it is asked from each, and
        // the `<parallel>` itself completes once.
        let parallel = Doc::new()
            .names("error.execution")
            .parallel("a", None)
            .state("r1", Some("a"))
            .state("r2", Some("a"))
            .state("b", None)
            .transition("a", guarded);
        assert_eq!(parallel.bound(), chain(3, 0));
    }

    /// The event that starts a macrostep can carry data its typed payload
    /// cannot be read from, which the engine reports with an error.
    #[test]
    fn a_typed_payload_that_cannot_be_lifted_counts() {
        let mut doc = Doc::new().names("error.execution").state("a", None);
        doc.model.imported_event_schemas.insert(
            "Ping".to_string(),
            crate::forge::model::EventSchemaModel {
                name: "Ping".to_string(),
                event_name: "ping".to_string(),
                fields: Vec::new(),
                payload_none: false,
                datamodel_markers: Vec::new(),
                revision: None,
                source_location: None,
            },
        );
        assert_eq!(doc.bound(), chain(1, 0));
    }

    #[test]
    fn a_send_to_the_internal_queue_counts_its_event() {
        let send = Action {
            action_type: "send".to_string(),
            event: "note".to_string(),
            target: "#_internal".to_string(),
            ..Action::default()
        };
        let doc = Doc::new().state("a", None).entry("a", vec![send]);
        assert_eq!(doc.bound(), chain(1, 0));
    }

    #[test]
    fn a_delayed_send_to_the_internal_queue_is_not_proven() {
        let send = Action {
            action_type: "send".to_string(),
            event: "note".to_string(),
            target: "#_internal".to_string(),
            delay: "1s".to_string(),
            delay_ms: 1000,
            ..Action::default()
        };
        let doc = Doc::new().state("a", None).entry("a", vec![send]);
        not_proven_because(&doc.bound(), "delayed");
    }

    #[test]
    fn a_script_engine_document_is_not_proven() {
        let mut doc = Doc::new().state("a", None);
        doc.model.needs_script_engine = true;
        not_proven_because(&doc.bound(), "script engine");
    }

    #[test]
    fn a_fallible_eventless_guard_is_not_proven() {
        let mut guarded = eventless("b");
        guarded.native_guard_fails = true;
        let doc = Doc::new()
            .names("error.execution")
            .state("a", None)
            .state("b", None)
            .transition("a", guarded);
        not_proven_because(&doc.bound(), "guard");
    }

    #[test]
    fn the_basis_is_written_in_lines_of_a_comment_width() {
        let bound = InternalQueueBound::NotProven {
            reason: "a reason long enough to need more than one line of a comment to hold all \
                     of the words it is written in"
                .to_string(),
        };
        let lines = bound.basis_lines();
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.len() <= 72));
        assert_eq!(lines.join(" "), bound.basis());
    }
}
