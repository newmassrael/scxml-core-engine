// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D, the entry half of a microstep: which states it
// enters, in which order, which of them enter by default, and whose
// `<history>` default content runs.
//
// The engines used to answer this one target at a time: enter the target's
// ancestor chain, then let each state descend into its own defaults as it was
// entered. That shape has room for exactly one target, so it could not say
// what a target SET enters (`target="a b"` across the regions of a
// `<parallel>`), what a recorded deep history spanning several regions
// restores, or that a compound state entered only as an ancestor of a deeper
// target has NOT been entered by default — its `<initial>` content must not
// run. The appendix computes the whole set first and enters it afterwards,
// sorted into document order, and every one of those answers falls out of
// that order of work rather than being a case of its own.
//
// The procedures are transcribed rather than paraphrased, one function per
// procedure and under the appendix's names, so a reader can hold this file
// against the specification line by line. What differs between engines is
// injected as one `Doc` describing the document: the Interpreter (string ids,
// `<history>` nodes in the same id space) and the generated code (enum
// states, histories in an enum of their own) each hand over their own, and
// both ask this one definition. The appendix's `getTransitionDomain` is not
// restated here — it is `ExitSetAlgorithms::getTransitionDomain`, the one the
// exit set and conflict resolution already use, fed the effective targets
// computed below.

#pragma once

#include "core/HierarchicalStateHelper.h"
#include "core/ParallelTransitionHelper.h"

#include <algorithm>
#include <optional>
#include <utility>
#include <vector>

namespace SCE::Core {

/**
 * @brief One token of a target list, as the document wrote it
 *
 * A transition's `target`, a state's initial transition, and a `<history>`
 * element's default transition all name either states or `<history>`
 * pseudo-states. A history is not a state — it is never in a configuration —
 * so the two kinds are kept apart rather than folded into one id: the entry
 * procedures dereference a history (to what it recorded, or to its default)
 * and add a state.
 */
template <typename State, typename History> class EntryTarget {
public:
    [[nodiscard]] static EntryTarget onState(State state) {
        return EntryTarget(std::move(state), History{}, false);
    }

    [[nodiscard]] static EntryTarget onHistory(History history) {
        return EntryTarget(State{}, std::move(history), true);
    }

    [[nodiscard]] bool isHistory() const noexcept {
        return isHistory_;
    }

    /// The state this token names. Meaningful only when `!isHistory()`.
    [[nodiscard]] const State &state() const noexcept {
        return state_;
    }

    /// The `<history>` this token names. Meaningful only when `isHistory()`.
    [[nodiscard]] const History &history() const noexcept {
        return history_;
    }

private:
    EntryTarget(State state, History history, bool isHistory)
        : state_(std::move(state)), history_(std::move(history)), isHistory_(isHistory) {}

    State state_;
    History history_;
    bool isHistory_;
};

/**
 * @brief The part of a transition the entry procedures read
 *
 * `source` is empty for the document's own initial transition, whose source
 * is the `<scxml>` element — which has no state identifier in either engine,
 * and whose domain is the whole document.
 */
template <typename State, typename History> struct EntryTransition {
    std::optional<State> source;
    std::vector<EntryTarget<State, History>> targets;
    bool isInternal = false;
};

/**
 * @brief A transition selection enabled: one member of Appendix D's
 *        `enabledTransitions`
 *
 * What the microstep reads of it — its source, its target list as written,
 * whether it is internal — plus the index its source state knows it by, so
 * the policy that owns its executable content can run it. A transition with
 * no targets exits and enters nothing and only runs its content.
 */
template <typename State, typename History> struct EnabledTransition {
    State source{};
    std::vector<EntryTarget<State, History>> targets;
    int transitionIndex = 0;
    bool hasActions = false;
    bool isInternal = false;

    [[nodiscard]] bool isTargetless() const noexcept {
        return targets.empty();
    }

    /// The same transition as the entry procedures see it.
    [[nodiscard]] EntryTransition<State, History> toEntryTransition() const {
        return EntryTransition<State, History>{source, targets, isInternal};
    }
};

/**
 * @brief What a microstep enters: the appendix's three out-parameters
 *
 * `statesToEnter` is already sorted into entry order, so an engine enters it
 * front to back. `statesForDefaultEntry` answers whether a state's initial
 * transition content runs; `defaultHistoryContent` whether, and whose,
 * `<history>` default transition content runs after that state's own entry.
 */
template <typename State, typename History> struct EntrySet {
    std::vector<State> statesToEnter;
    std::vector<State> statesForDefaultEntry;
    /// Keyed by the history's parent, as the appendix keys it: the content
    /// runs when that parent is entered.
    std::vector<std::pair<State, History>> defaultHistoryContent;

    [[nodiscard]] bool contains(const State &state) const {
        return std::find(statesToEnter.begin(), statesToEnter.end(), state) != statesToEnter.end();
    }

    /// Whether `state`'s initial state is being entered by default, so its
    /// initial transition's executable content runs.
    [[nodiscard]] bool isDefaultEntry(const State &state) const {
        return std::find(statesForDefaultEntry.begin(), statesForDefaultEntry.end(), state) !=
               statesForDefaultEntry.end();
    }

    /// The `<history>` whose default transition content runs after `state`
    /// is entered, if a history of `state` was taken with nothing recorded.
    [[nodiscard]] std::optional<History> defaultHistoryContentOf(const State &state) const {
        for (const auto &entry : defaultHistoryContent) {
            if (entry.first == state) {
                return entry.second;
            }
        }
        return std::nullopt;
    }
};

/**
 * @brief Appendix D's computeEntrySet and the procedures it calls
 *
 * Every procedure takes a `Doc`, which describes the document and the
 * current history values. It must provide:
 *
 * ```
 * using State = ...;    // a state identifier
 * using History = ...;  // a <history> identifier
 * std::optional<State> parentOf(const State &) const;  // nullopt at the top
 * bool isCompound(const State &) const;  // a <state> with child states;
 *                                        // a <parallel> answers false
 * bool isParallel(const State &) const;
 * std::vector<State> childStates(const State &) const;  // getChildStates:
 *     // <state>, <parallel> and <final> children in document order
 * std::vector<EntryTarget<State, History>> initialTargets(const State &) const;
 *     // a compound state's initial transition target, as written; the
 *     // first child state when the document names none
 * State historyParent(const History &) const;
 * std::optional<std::vector<State>> historyValue(const History &) const;
 *     // what the history recorded; nullopt (or empty) before its parent
 *     // was ever exited
 * std::vector<EntryTarget<State, History>> historyDefaultTargets(const History &) const;
 * int documentOrder(const State &) const;
 * ```
 *
 * A document whose `<history>` defaults name one another in a cycle has no
 * entry set — dereferencing never reaches a state — so it must be refused
 * before anything here runs; the procedures assume a legal document.
 */
struct EntrySetAlgorithms {
    template <typename Doc> using Target = EntryTarget<typename Doc::State, typename Doc::History>;
    template <typename Doc> using Transition = EntryTransition<typename Doc::State, typename Doc::History>;
    template <typename Doc> using Set = EntrySet<typename Doc::State, typename Doc::History>;

    /**
     * @brief Appendix D's getEffectiveTargetStates, over a target list
     *
     * @return The states the list stands for, histories dereferenced, each
     *         state once and in the order first named
     */
    template <typename Doc>
    [[nodiscard]] static std::vector<typename Doc::State>
    getEffectiveTargetStates(const std::vector<Target<Doc>> &targets, const Doc &doc) {
        std::vector<typename Doc::State> effective;
        for (const auto &target : targets) {
            // §scxml-D-getEffectiveTargetStates: a history stands for the
            // configuration it recorded, and before its parent was ever
            // exited for the targets of its default transition — which may
            // name histories themselves, hence the recursion.
            if (!target.isHistory()) {
                addOnce(effective, target.state());
                continue;
            }
            const auto recorded = doc.historyValue(target.history());
            if (recorded.has_value() && !recorded->empty()) {
                for (const auto &state : *recorded) {
                    addOnce(effective, state);
                }
            } else {
                for (const auto &state : getEffectiveTargetStates(doc.historyDefaultTargets(target.history()), doc)) {
                    addOnce(effective, state);
                }
            }
        }
        return effective;
    }

    /**
     * @brief Appendix D's getTransitionDomain, for a transition that has targets
     *
     * @return The domain, or std::nullopt when it is the `<scxml>` element —
     *         always so for the document's initial transition
     */
    template <typename Doc>
    [[nodiscard]] static std::optional<typename Doc::State> getTransitionDomain(const Transition<Doc> &transition,
                                                                                const Doc &doc) {
        if (!transition.source.has_value()) {
            return std::nullopt;
        }
        // The domain is asked of the EFFECTIVE targets: a history that
        // recorded a state deep inside its parent is a target that deep.
        return ExitSetAlgorithms::getTransitionDomain(
            *transition.source, getEffectiveTargetStates(transition.targets, doc), transition.isInternal,
            [&doc](const typename Doc::State &s) { return doc.parentOf(s); },
            [&doc](const typename Doc::State &s) { return doc.isCompound(s); });
    }

    /**
     * @brief Appendix D's computeEntrySet
     *
     * @param transitions The microstep's transitions, in the order they were
     *        selected. A transition without targets enters nothing.
     * @return The entry set, `statesToEnter` sorted into entry order
     */
    template <typename Doc>
    [[nodiscard]] static Set<Doc> computeEntrySet(const std::vector<Transition<Doc>> &transitions, const Doc &doc) {
        Set<Doc> entry;
        for (const auto &transition : transitions) {
            // §scxml-D-computeEntrySet: first every target with its default
            // descendants, then the ancestors that are entered inside the
            // domain — ancestors outside it were never exited. The order
            // matters for a target SET: the regions of a `<parallel>` that
            // another target already descends into must be seen as taken
            // before the ancestor walk fills the rest with defaults.
            for (const auto &target : transition.targets) {
                addDescendantStatesToEnter(target, doc, entry);
            }
            const auto effective = getEffectiveTargetStates(transition.targets, doc);
            if (effective.empty()) {
                continue;
            }
            const auto domain = getTransitionDomain(transition, doc);
            for (const auto &state : effective) {
                addAncestorStatesToEnter(Target<Doc>::onState(state), domain, doc, entry);
            }
        }

        // §scxml-D-enterStates: states are entered in entryOrder — ancestors
        // before descendants, document order between the rest, which is
        // exactly document order, because that is a pre-order walk.
        std::stable_sort(entry.statesToEnter.begin(), entry.statesToEnter.end(),
                         [&doc](const typename Doc::State &a, const typename Doc::State &b) {
                             return doc.documentOrder(a) < doc.documentOrder(b);
                         });
        return entry;
    }

    /**
     * @brief Appendix D's addDescendantStatesToEnter
     *
     * Adds `target` and every descendant entering it enters: a history's
     * recorded or default configuration, a compound state's initial
     * state(s), every region of a `<parallel>` not already on the set.
     */
    template <typename Doc>
    static void addDescendantStatesToEnter(const Target<Doc> &target, const Doc &doc, Set<Doc> &entry) {
        if (target.isHistory()) {
            const auto &history = target.history();
            const auto parent = doc.historyParent(history);
            const auto recorded = doc.historyValue(history);
            // §scxml-3.10: a transition to a history behaves as a transition
            // to the configuration it stored, or — before its parent was ever
            // visited — to its default stored configuration.
            if (recorded.has_value() && !recorded->empty()) {
                // §scxml-D-addDescendantStatesToEnter: a history that has
                // recorded a configuration enters it, and the ancestors
                // between it and the history's parent — a deep history
                // records atomic states, which need not be children.
                for (const auto &state : *recorded) {
                    addDescendantStatesToEnter(Target<Doc>::onState(state), doc, entry);
                }
                for (const auto &state : *recorded) {
                    addAncestorStatesToEnter(Target<Doc>::onState(state), parent, doc, entry);
                }
            } else {
                // §scxml-D-addDescendantStatesToEnter: nothing recorded yet,
                // so the default transition is taken, and its content is
                // owed once the parent has been entered — keyed by the
                // parent, a later history of the same parent replacing it.
                setDefaultHistoryContent(entry, parent, history);
                const auto defaults = doc.historyDefaultTargets(history);
                for (const auto &state : defaults) {
                    addDescendantStatesToEnter(state, doc, entry);
                }
                for (const auto &state : defaults) {
                    addAncestorStatesToEnter(state, parent, doc, entry);
                }
            }
            return;
        }

        const auto &state = target.state();
        addOnce(entry.statesToEnter, state);
        if (doc.isCompound(state)) {
            // §scxml-D-addDescendantStatesToEnter: a compound state entered
            // as a target is entered by DEFAULT — the one condition under
            // which its initial transition's content runs. Its initial
            // transition names the child or children it enters (§scxml-3.3),
            // possibly several and possibly deep (§scxml-3.6).
            addOnce(entry.statesForDefaultEntry, state);
            const auto initial = doc.initialTargets(state);
            for (const auto &child : initial) {
                addDescendantStatesToEnter(child, doc, entry);
            }
            for (const auto &child : initial) {
                addAncestorStatesToEnter(child, state, doc, entry);
            }
        } else if (doc.isParallel(state)) {
            addRegionDefaults(state, doc, entry);
        }
    }

    /**
     * @brief Appendix D's addAncestorStatesToEnter
     *
     * Adds the proper ancestors of `target` up to, not including,
     * `ancestor` (`nullopt`: up to the `<scxml>` element), filling in the
     * regions of every `<parallel>` among them.
     */
    template <typename Doc>
    static void addAncestorStatesToEnter(const Target<Doc> &target, const std::optional<typename Doc::State> &ancestor,
                                         const Doc &doc, Set<Doc> &entry) {
        for (const auto &anc : getProperAncestors(target, ancestor, doc)) {
            // §scxml-D-addAncestorStatesToEnter: an ancestor is entered
            // WITHOUT its default initial state — the set already holds the
            // descendant it leads to. A `<parallel>` still gives its other
            // regions their defaults, because all of them are entered.
            addOnce(entry.statesToEnter, anc);
            if (doc.isParallel(anc)) {
                addRegionDefaults(anc, doc, entry);
            }
        }
    }

private:
    template <typename Doc>
    static std::vector<typename Doc::State>
    getProperAncestors(const Target<Doc> &target, const std::optional<typename Doc::State> &ancestor, const Doc &doc) {
        // §scxml-D-getProperAncestors: the ancestors in ancestry order up to,
        // not including, `ancestor` — and nothing at all when `ancestor` is
        // not above the state. A `<history>` sits in its parent, so the
        // parent is its first proper ancestor.
        std::vector<typename Doc::State> chain;
        std::optional<typename Doc::State> current =
            target.isHistory() ? std::optional<typename Doc::State>(doc.historyParent(target.history()))
                               : doc.parentOf(target.state());
        while (current.has_value()) {
            if (ancestor.has_value() && *current == *ancestor) {
                return chain;
            }
            chain.push_back(*current);
            current = doc.parentOf(*current);
        }
        if (ancestor.has_value()) {
            chain.clear();
        }
        return chain;
    }

    template <typename Doc>
    static void addRegionDefaults(const typename Doc::State &parallel, const Doc &doc, Set<Doc> &entry) {
        // §scxml-3.4: every child of an active <parallel> is active, so a
        // region nothing on the set descends into is entered by default.
        const auto parentOf = [&doc](const typename Doc::State &s) { return doc.parentOf(s); };
        for (const auto &child : doc.childStates(parallel)) {
            const bool taken =
                std::any_of(entry.statesToEnter.begin(), entry.statesToEnter.end(), [&](const typename Doc::State &s) {
                    return HierarchicalAlgorithms::isDescendantOf(s, child, parentOf);
                });
            if (!taken) {
                addDescendantStatesToEnter(Target<Doc>::onState(child), doc, entry);
            }
        }
    }

    template <typename State, typename History>
    static void setDefaultHistoryContent(EntrySet<State, History> &entry, const State &parent, const History &history) {
        for (auto &slot : entry.defaultHistoryContent) {
            if (slot.first == parent) {
                slot.second = history;
                return;
            }
        }
        entry.defaultHistoryContent.emplace_back(parent, history);
    }

    template <typename State> static void addOnce(std::vector<State> &set, const State &state) {
        if (std::find(set.begin(), set.end(), state) == set.end()) {
            set.push_back(state);
        }
    }
};

}  // namespace SCE::Core
