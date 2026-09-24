// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D's microstep — which transitions an event selects, which
// of them survive preemption, which states they exit and in which order, the
// order their content runs in, which states they enter — written once for both
// C++ engines.
//
// The engines differ in what a state IS (an enum of the generated policy, a
// string id of the parsed model) and in how content runs (generated code,
// `IActionNode` trees). They must not differ in what the appendix does with
// those answers, and they did: the AOT engine ran the appendix's microstep
// while the Interpreter chose between three hand-written ones by the shape of
// the configuration, and those three disagreed with the appendix and with one
// another about which transitions an event selects, which states a transition
// exits, the order content runs in, and how many targets a transition has.
//
// So each procedure is written here once, under the appendix's name, and each
// engine hands over a `Host`: the document, as `EntrySetAlgorithms` reads it,
// and the run — the configuration, which of a state's transitions an event
// enables, and what exiting a state, entering one and taking a transition DO in
// that engine. What this orchestrates is what both engines already share:
// `ExitSetAlgorithms` for domains and exit sets, `ConflictResolutionAlgorithms`
// for preemption, `EntrySetAlgorithms` for the entry set.

#pragma once

#include "core/ConflictResolutionHelper.h"
#include "core/EntrySetHelper.h"
#include "core/ParallelTransitionHelper.h"

#include <algorithm>
#include <optional>
#include <utility>
#include <vector>

namespace SCE::Core {

/**
 * @brief Appendix D's microstep and the procedures it calls
 *
 * Every procedure takes a `Host`. It provides the document — exactly the
 * `Doc` contract `EntrySetAlgorithms` states, so a Host IS a Doc — and the run:
 *
 * ```
 * std::vector<State> configuration() const;
 *     // the active states, in any order
 * std::optional<EnabledTransition<State, History>> firstEnabledTransition(const State &, const Event &);
 *     // this state's first transition, in document order, that the event
 *     // enables and whose guard holds; for the host's "no event", its first
 *     // eventless transition whose guard holds. The only place a guard is
 *     // evaluated, so each is evaluated once per selection.
 * void exitState(const State &, const std::vector<State> &configurationBeforeExit);
 *     // record this state's histories from the configuration as it stood
 *     // before the microstep's first exit, run its onexit, cancel its
 *     // invocations, remove it from the configuration
 * void executeTransitionContent(const EnabledTransition<State, History> &);
 * void enterState(const State &, bool isDefaultEntry);
 *     // add it to the configuration and schedule its invocations, run its
 *     // onentry, then its initial transition's content when `isDefaultEntry`;
 *     // for a <final>, what the appendix does on entering one
 * void executeHistoryDefaultContent(const History &);
 * ```
 *
 * Event is the host's own type; this file only hands it back.
 */
struct MicrostepAlgorithms {
    template <typename Host> using Transition = EnabledTransition<typename Host::State, typename Host::History>;
    template <typename Host> using Entering = EntryTransition<typename Host::State, typename Host::History>;
    template <typename Host> using Entered = EntrySet<typename Host::State, typename Host::History>;

    /**
     * @brief A transition's targets with every `<history>` dereferenced — to
     *        what it recorded or, before its parent was ever exited, to its
     *        default. The domain, and so the exit set, is a question about these.
     */
    template <typename Host>
    [[nodiscard]] static std::vector<typename Host::State> effectiveTargets(const Host &host,
                                                                            const Transition<Host> &transition) {
        return EntrySetAlgorithms::getEffectiveTargetStates(transition.targets, host);
    }

    /**
     * @brief Appendix D's selectTransitions, or its selectEventlessTransitions
     *        when @p event is the host's "no event"
     *
     * @return The optimal enabled transition set, in selection order
     */
    template <typename Host, typename Event>
    [[nodiscard]] static std::vector<Transition<Host>> selectTransitions(Host &host, const Event &event) {
        using State = typename Host::State;
        const std::vector<State> configuration = host.configuration();

        std::vector<State> atomicStates;
        for (const State &s : configuration) {
            if (!host.isCompound(s) && !host.isParallel(s)) {
                atomicStates.push_back(s);
            }
        }
        std::sort(atomicStates.begin(), atomicStates.end(),
                  [&host](const State &a, const State &b) { return host.documentOrder(a) < host.documentOrder(b); });

        std::vector<Transition<Host>> enabled;
        for (const State &atomic : atomicStates) {
            // §scxml-D-selectTransitions: the atomic state first, then its
            // proper ancestors, and the first enabled transition in document
            // order ends the walk for this atomic state. The set is ORDERED
            // and a set: two atomic states under one ancestor both reach its
            // transition, and it is one transition, taken once. With the host's
            // "no event" this is §scxml-D-selectEventlessTransitions, the same
            // walk over transitions that have no event.
            for (std::optional<State> s = atomic; s.has_value(); s = host.parentOf(*s)) {
                std::optional<Transition<Host>> found = host.firstEnabledTransition(*s, event);
                if (!found.has_value()) {
                    continue;
                }
                const bool seen = std::any_of(enabled.begin(), enabled.end(), [&found](const Transition<Host> &t) {
                    return t.source == found->source && t.transitionIndex == found->transitionIndex;
                });
                if (!seen) {
                    enabled.push_back(std::move(*found));
                }
                break;
            }
        }
        return removeConflictingTransitions(host, enabled, configuration);
    }

    /**
     * @brief Appendix D's removeConflictingTransitions
     */
    template <typename Host>
    [[nodiscard]] static std::vector<Transition<Host>>
    removeConflictingTransitions(const Host &host, const std::vector<Transition<Host>> &enabled,
                                 const std::vector<typename Host::State> &configuration) {
        using State = typename Host::State;
        if (enabled.size() < 2) {
            return enabled;
        }
        // §scxml-D-removeConflictingTransitions: two transitions conflict when
        // their exit sets intersect, and an exit set is read off the
        // CONFIGURATION over the transition's EFFECTIVE targets — a `<history>`
        // target is as deep as what it recorded.
        using Descriptor = ConflictResolutionAlgorithms::TransitionDescriptor<State>;
        std::vector<Descriptor> descriptors;
        descriptors.reserve(enabled.size());
        for (const Transition<Host> &t : enabled) {
            Descriptor d(t.source, effectiveTargets(host, t), t.transitionIndex, t.hasActions, t.isInternal,
                         t.isTargetless());
            d.exitSet = ExitSetAlgorithms::computeExitSet(d.source, d.targets, d.isInternal, d.isTargetless,
                                                          configuration, parentOf(host), isCompound(host));
            descriptors.push_back(std::move(d));
        }
        const std::vector<Descriptor> kept =
            ConflictResolutionAlgorithms::removeConflictingTransitions(descriptors, parentOf(host));

        std::vector<Transition<Host>> result;
        for (const Transition<Host> &t : enabled) {
            const bool survives = std::any_of(kept.begin(), kept.end(), [&t](const Descriptor &d) {
                return d.source == t.source && d.transitionIndex == t.transitionIndex;
            });
            if (survives) {
                result.push_back(t);
            }
        }
        return result;
    }

    /**
     * @brief Appendix D's computeExitSet over the microstep's transitions, in
     *        exitOrder — the states `exitStates` exits
     */
    template <typename Host>
    [[nodiscard]] static std::vector<typename Host::State>
    computeStatesToExit(const Host &host, const std::vector<Transition<Host>> &transitions,
                        const std::vector<typename Host::State> &configuration) {
        using State = typename Host::State;
        std::vector<ParallelTransitionHelper::Transition<State>> views;
        views.reserve(transitions.size());
        for (const Transition<Host> &t : transitions) {
            views.emplace_back(t.source, effectiveTargets(host, t), t.transitionIndex, t.hasActions, t.isInternal,
                               t.isTargetless());
        }
        return ExitSetAlgorithms::computeStatesToExit(views, configuration, parentOf(host), isCompound(host),
                                                      [&host](const State &s) { return host.documentOrder(s); });
    }

    /**
     * @brief Appendix D's microstep: exit, run the transitions' content, enter
     *
     * @return What was entered — the entry set, `statesToEnter` in entry order
     */
    template <typename Host>
    static Entered<Host> microstep(Host &host, const std::vector<Transition<Host>> &transitions) {
        // §scxml-D-microstepProcedure: every exit, then every transition's
        // content, then every entry — for the whole set at once, which is what
        // lets the regions of a `<parallel>` each take their own transition in
        // one step.
        exitStates(host, transitions);
        executeTransitionContent(host, transitions);
        std::vector<Entering<Host>> entering;
        entering.reserve(transitions.size());
        for (const Transition<Host> &t : transitions) {
            entering.push_back(t.toEntryTransition());
        }
        return enterStates(host, entering);
    }

    template <typename Host> static void exitStates(Host &host, const std::vector<Transition<Host>> &transitions) {
        // §scxml-D-exitStates: the union of the transitions' exit sets, exited
        // in exitOrder. Every history is recorded from the configuration as it
        // stood BEFORE the first exit, which is the snapshot each exit is
        // handed.
        const std::vector<typename Host::State> configuration = host.configuration();
        for (const auto &s : computeStatesToExit(host, transitions, configuration)) {
            host.exitState(s, configuration);
        }
    }

    template <typename Host>
    static void executeTransitionContent(Host &host, const std::vector<Transition<Host>> &transitions) {
        // §scxml-D-executeTransitionContent: in the order the transitions were
        // selected, which is not their sources' document order once an
        // ancestor's transition is reached from a later region.
        for (const Transition<Host> &t : transitions) {
            if (t.hasActions) {
                host.executeTransitionContent(t);
            }
        }
    }

    /**
     * @brief Appendix D's enterStates
     *
     * Also the whole of the appendix's entry into the initial configuration
     * (`interpret`): hand it the document's initial transition, whose source is
     * the `<scxml>` element (`source` empty).
     */
    template <typename Host>
    static Entered<Host> enterStates(Host &host, const std::vector<Entering<Host>> &transitions) {
        Entered<Host> entry = EntrySetAlgorithms::computeEntrySet(transitions, host);
        for (const auto &s : entry.statesToEnter) {
            // §scxml-D-enterStates: onentry, then the initial transition's
            // content if and only if this state's initial state is being
            // entered by default, then a history's default content owed to it.
            host.enterState(s, entry.isDefaultEntry(s));
            if (const auto history = entry.defaultHistoryContentOf(s)) {
                host.executeHistoryDefaultContent(*history);
            }
        }
        return entry;
    }

private:
    template <typename Host> static auto parentOf(const Host &host) {
        return [&host](const typename Host::State &s) { return host.parentOf(s); };
    }

    template <typename Host> static auto isCompound(const Host &host) {
        return [&host](const typename Host::State &s) { return host.isCompound(s); };
    }
};

}  // namespace SCE::Core
