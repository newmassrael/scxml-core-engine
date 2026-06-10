// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE
//
// SCE Forge: event-driven execution loop for Level 2 procedure state
// machines. Generated procedure code calls run_procedure() to drive the
// loop; the policy struct supplies the step methods in a return-event
// shape identical to the Rust / Python / Kotlin / Go runtimes in this
// crate. No engine parameter is threaded into the policy, no event
// queue is involved — each entry action returns the single event that
// was "raised" by its sole <send sce:service> (if any).

#pragma once

#include <cstddef>
#include <optional>
#include <string>
#include <tuple>
#include <utility>

#include "sce/forge/ProcedureServiceTypes.h"

namespace SCE::Forge {

/// Safety cap shared with Rust/Python/Kotlin/Go run loops — bounds the
/// iteration count so that a policy with an unreachable final state
/// cannot spin forever.
inline constexpr int kProcedureMaxIterations = 1000;

/// Drive a Level 2 procedure policy to completion.
///
/// `Policy` is the generated state-machine class. It must expose the
/// same shape as the Rust `ProcedurePolicy` trait:
///
///   // Type aliases
///   using State = ...;
///   using Event = ...;
///
///   // Static metadata — no instance state touched.
///   static constexpr State initialState() noexcept;
///   static constexpr Event noneEvent() noexcept;
///   static constexpr bool isFinalState(State) noexcept;
///   static constexpr const char* finalStateName(State) noexcept;
///
///   // Stateful step methods — called in the same order as Rust's
///   // run_procedure(), Python's run_to_completion(), Kotlin's
///   // runToCompletion(), and Go's RunProcedure().
///   std::pair<Event, std::string> executeEntryActions(State);
///   std::optional<std::tuple<State, std::size_t, bool>>
///       processTransition(State, Event) const;
///   std::optional<Event> executeTransitionActions(State source, std::size_t trIndex);
///
///   // Engine-visible datamodel slots.
///   void setPendingEventData(std::string);
///   const std::map<std::string, std::string>& doneData() const;
///
/// The loop executes entry actions on the initial state, then iterates:
/// check isFinalState → processTransition → executeTransitionActions
/// (when hasAssigns) → entry actions of the new state. The next event
/// is the one returned by the latest entry-action call; an empty event
/// (Event equal to `noneEvent()`) drives eventless transitions.
///
/// `executeTransitionActions` returns `std::nullopt` for normal flow
/// (transition completes and the loop advances to the target state).
/// A non-empty return signals that an assign-time check (e.g. a
/// bytes cap violation)
/// fired; the loop re-processes the *source* state with that event so
/// the fixture's `<transition event="error.execution">` (or any other
/// internal-event handler) can pick it up. If the source state has no
/// matching transition, `processTransition` returns `nullopt` and the
/// procedure terminates uncompleted — W3C-correct for an unhandled
/// internal event.
template <typename Policy>
ProcedureRunResult run_procedure(Policy &policy) {
    using State = typename Policy::State;
    using Event = typename Policy::Event;

    State current = Policy::initialState();

    auto initialStep = policy.executeEntryActions(current);
    Event event = initialStep.first;
    if (!initialStep.second.empty()) {
        policy.setPendingEventData(std::move(initialStep.second));
    }

    for (int i = 0; i < kProcedureMaxIterations; ++i) {
        if (Policy::isFinalState(current)) {
            break;
        }
        auto transition = policy.processTransition(current, event);
        if (!transition.has_value()) {
            break;
        }
        State next;
        std::size_t trIndex;
        bool hasAssigns;
        std::tie(next, trIndex, hasAssigns) = *transition;
        if (hasAssigns) {
            auto raised = policy.executeTransitionActions(current, trIndex);
            if (raised.has_value()) {
                // Assign-time check raised an internal event; re-process the
                // source state with it instead of advancing to `next`. The
                // source's transitions decide whether to handle the event
                // (e.g. `<transition event="error.execution" target="error"/>`)
                // or let `processTransition` return nullopt next iteration so
                // the procedure terminates uncompleted.
                event = *raised;
                continue;
            }
        }
        current = next;
        event = Policy::noneEvent();
        auto step = policy.executeEntryActions(current);
        event = step.first;
        if (!step.second.empty()) {
            policy.setPendingEventData(std::move(step.second));
        }
    }

    ProcedureRunResult result;
    result.completed = Policy::isFinalState(current);
    if (result.completed) {
        result.final_state = Policy::finalStateName(current);
        result.done_data = policy.doneData();
    }
    return result;
}

}  // namespace SCE::Forge
