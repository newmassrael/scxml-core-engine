// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A document printed as its own step function (docs/adr/0129).
//!
//! A document of the static class has no `<parallel>`, no `<history>`, no
//! `<invoke>`, no transition of more than one target and an internal queue
//! whose depth the generator proves. Its configuration is one atomic state and
//! that state's ancestors, a selection takes one transition at most, and what
//! each transition exits and enters from a given atomic state is fixed when
//! the document is built (docs/adr/0125). Such a document is printed one of
//! two ways, and the build that generates the machines of an image says which:
//!
//! - for the shared interpreter, whose main loop, selection and walks are one
//!   copy in the program and reach the machine through `dyn` hooks
//!   (docs/adr/0064, docs/adr/0068): the table-driven form;
//! - as its own step function, which this module runs: the generated policy
//!   implements [`StepFunction`], whose arms write out what each transition
//!   exits and enters from each atomic state as constants, and the main loop
//!   is compiled for the machine with every hook a direct call: the directly
//!   coded form.
//!
//! The two forms run one main loop, written once (`engine::Turn`), and call
//! the same hooks of the generated policy for every guard, every `<onexit>`,
//! every transition's content and every `<onentry>`. What differs is how the
//! enabled transition is found and how its sets are walked: by the shared
//! procedures over the tables, or by the arms the generator printed. The
//! differential test of docs/adr/0129 drives both forms of one document with
//! the same events and holds every observation of the one to the other.
//!
//! A policy chooses by its `StatePolicy::STEP_FUNCTION`: `None`, the default,
//! for the shared interpreter, and [`StepFunctionForm::of`] for its own step
//! function. The engine reads the constant where it builds a turn, so a machine
//! of either form names only the code of its own form, and a program none of
//! whose machines is interpreted links none of the shared loop.

use super::{EngineHost, Form};
use crate::helpers::microstep::{Behaviour, Entered, Scratch, SliceVec};
use crate::helpers::tables::{DocumentTables, EventId, StateId, TransitionId};
use crate::policy::StatePolicy;

/// The running machine, as a step function drives it: the four things the
/// arms of a printed step function ask of the engine. Implemented by the
/// engine alone; a generated policy calls it and never implements it.
///
/// Each is the same call the shared procedures make for the same step, with
/// the same hook of the generated policy behind it, so the two forms of one
/// document differ in the order of nothing they do.
pub trait Steps {
    /// The first transition of `state`, in document order, that `event`
    /// enables and whose guard holds; for the machine's "no event", its first
    /// eventless transition whose guard holds. The guard is the generated
    /// policy's, evaluated here and nowhere else.
    fn enabled(&mut self, state: StateId, event: EventId) -> Option<TransitionId>;

    /// Exit `state`: its `<onexit>`, then it leaves the configuration, so a
    /// test of whether it is active inside its own handler holds.
    fn exit(&mut self, state: StateId);

    /// Run the executable content of `transition`.
    fn content(&mut self, transition: TransitionId);

    /// Enter `state`: it joins the configuration, then its `<onentry>` runs,
    /// and its initial transition's content after that when `by_default`. A
    /// state already active is not entered again.
    fn enter(&mut self, state: StateId, by_default: bool);
}

/// A document's step function: what the generator prints for a document of
/// the static class when the build asks for its own step function
/// (docs/adr/0129). Each method is one `match` whose arms the generator wrote
/// from the document, and each arm a sequence of [`Steps`] calls.
///
/// `atomic` is always the configuration's one atomic state, which the engine
/// reads from the configuration it keeps: the state of the largest number in
/// it, since every other active state is one of its ancestors and an ancestor
/// is numbered before its descendants.
pub trait StepFunction: StatePolicy {
    /// Enter the initial configuration: every state the document's initial
    /// transition enters, in entry order. The configuration is empty.
    fn enter_initial_configuration<M: Steps>(machine: &mut M) -> Entered;

    /// The transition `event` enables in the configuration whose atomic state
    /// is `atomic`: that state's own transitions first, then each ancestor's,
    /// outward, the first enabled one ending the search.
    fn select_transition<M: Steps>(
        machine: &mut M,
        atomic: StateId,
        event: EventId,
    ) -> Option<TransitionId>;

    /// Take `transition`, which [`select_transition`](Self::select_transition)
    /// selected in the configuration whose atomic state is `atomic`: every
    /// active state below its domain exited, atomic state first, then its
    /// content, then every state of its entry path entered.
    fn microstep<M: Steps>(machine: &mut M, atomic: StateId, transition: TransitionId) -> Entered;

    /// Exit every active state, atomic state first, as the run ends.
    fn exit_configuration<M: Steps>(machine: &mut M, atomic: StateId);
}

/// The initial entry of a step function, as the engine holds it.
type EnterInitial<P> = for<'h, 'e, 'd> fn(&'h mut EngineHost<'e, 'd, P>) -> Entered;
/// The selection of a step function, as the engine holds it.
type SelectTransition<P> =
    for<'h, 'e, 'd> fn(&'h mut EngineHost<'e, 'd, P>, StateId, EventId) -> Option<TransitionId>;
/// The microstep of a step function, as the engine holds it.
type Microstep<P> =
    for<'h, 'e, 'd> fn(&'h mut EngineHost<'e, 'd, P>, StateId, TransitionId) -> Entered;
/// The final exit of a step function, as the engine holds it.
type ExitConfiguration<P> = for<'h, 'e, 'd> fn(&'h mut EngineHost<'e, 'd, P>, StateId);

/// How a policy is run when it is printed as its own step function: its
/// [`StepFunction`], held as the four procedures the engine calls. A policy
/// names it as `StatePolicy::STEP_FUNCTION = Some(StepFunctionForm::of())`;
/// the fields are private, so [`of`](Self::of) is the only value there is.
///
/// The engine reads it from a constant of the policy, so every call through
/// it is to a function known where the engine is compiled for the machine.
pub struct StepFunctionForm<P: StatePolicy> {
    enter_initial_configuration: EnterInitial<P>,
    select_transition: SelectTransition<P>,
    microstep: Microstep<P>,
    exit_configuration: ExitConfiguration<P>,
}

impl<P: StatePolicy> Clone for StepFunctionForm<P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P: StatePolicy> Copy for StepFunctionForm<P> {}

impl<P: StatePolicy> core::fmt::Debug for StepFunctionForm<P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("StepFunctionForm")
    }
}

impl<P: StepFunction> StepFunctionForm<P> {
    /// The step function `P` implements, as the engine runs it.
    pub const fn of() -> Self {
        Self {
            enter_initial_configuration: enter_initial_configuration::<P>,
            select_transition: select_transition::<P>,
            microstep: microstep::<P>,
            exit_configuration: exit_configuration::<P>,
        }
    }
}

// What each field holds: the policy's own procedure, over this engine. A
// function of its own and not the trait method itself, so the engine's
// lifetimes are the pointer's to choose at each call.

fn enter_initial_configuration<P: StepFunction>(host: &mut EngineHost<'_, '_, P>) -> Entered {
    P::enter_initial_configuration(host)
}

fn select_transition<P: StepFunction>(
    host: &mut EngineHost<'_, '_, P>,
    atomic: StateId,
    event: EventId,
) -> Option<TransitionId> {
    P::select_transition(host, atomic, event)
}

fn microstep<P: StepFunction>(
    host: &mut EngineHost<'_, '_, P>,
    atomic: StateId,
    transition: TransitionId,
) -> Entered {
    P::microstep(host, atomic, transition)
}

fn exit_configuration<P: StepFunction>(host: &mut EngineHost<'_, '_, P>, atomic: StateId) {
    P::exit_configuration(host, atomic);
}

impl<P: StatePolicy> Steps for EngineHost<'_, '_, P> {
    #[inline]
    fn enabled(&mut self, state: StateId, event: EventId) -> Option<TransitionId> {
        Behaviour::first_enabled_transition(self, state, event)
    }

    #[inline]
    fn exit(&mut self, state: StateId) {
        Behaviour::exit_state(self, state);
        Behaviour::store(self).remove(state);
    }

    #[inline]
    fn content(&mut self, transition: TransitionId) {
        Behaviour::execute_transition_content(self, transition);
    }

    #[inline]
    fn enter(&mut self, state: StateId, by_default: bool) {
        if Behaviour::store(self).add(state) {
            Behaviour::enter_state(self, state, by_default);
        }
    }
}

/// The configuration's atomic state: its last state in document order, every
/// other active state being one of that state's ancestors. `None` for the
/// empty configuration, before the run is entered and after it is exited.
fn atomic_state<P: StatePolicy>(host: &mut EngineHost<'_, '_, P>) -> Option<StateId> {
    Behaviour::store(host).as_ref().configuration().last()
}

/// The step function of a machine printed as its own, run by the main loop
/// every machine shares the source of. The tables and the scratch are the
/// shared procedures' and are read by none of these: a step function's sets
/// are its arms.
impl<'e, 'd, P: StatePolicy> Form<EngineHost<'e, 'd, P>> for StepFunctionForm<P> {
    fn enter_initial_configuration(
        &self,
        host: &mut EngineHost<'e, 'd, P>,
        _tables: &DocumentTables<'_>,
        _scratch: Scratch<'_>,
    ) -> Entered {
        Behaviour::store(host).configuration().clear();
        (self.enter_initial_configuration)(host)
    }

    fn select_transitions(
        &self,
        host: &mut EngineHost<'e, 'd, P>,
        _tables: &DocumentTables<'_>,
        event: EventId,
        enabled: &mut SliceVec<'_, TransitionId>,
        _scratch: Scratch<'_>,
    ) {
        enabled.clear();
        let Some(atomic) = atomic_state(host) else {
            return;
        };
        if let Some(transition) = (self.select_transition)(host, atomic, event) {
            enabled.push(transition);
        }
    }

    fn microstep(
        &self,
        host: &mut EngineHost<'e, 'd, P>,
        _tables: &DocumentTables<'_>,
        enabled: &[TransitionId],
        _scratch: Scratch<'_>,
    ) -> Entered {
        // A test component's branch takes what it matched before anything
        // exits (docs/SCE_TEST_PROFILE.md section 4.4), as the shared
        // microstep has it.
        #[cfg(feature = "test-component")]
        for &transition in enabled {
            Behaviour::take_transition(host, transition);
        }
        // The selection enabled one transition, from the atomic state the
        // configuration still holds: nothing has exited yet.
        match (enabled.first(), atomic_state(host)) {
            (Some(&transition), Some(atomic)) => (self.microstep)(host, atomic, transition),
            _ => Entered {
                states: 0,
                current: None,
                ended_in: None,
            },
        }
    }

    fn exit_configuration(&self, host: &mut EngineHost<'e, 'd, P>) {
        if let Some(atomic) = atomic_state(host) {
            (self.exit_configuration)(host, atomic);
        }
    }
}
