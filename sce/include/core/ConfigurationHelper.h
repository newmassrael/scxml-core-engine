// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-3.11: is this set of states a CONFIGURATION of the document, and is
// `current` its current state?
//
// A host that persisted where a machine was and is bringing it back in a new
// process hands the engine a set of states it read off a journal. Entering
// "near" the requested configuration is the one outcome that door must never
// produce, because nothing afterwards can detect it — the machine reports
// itself running, `getCurrentState` answers, and the configuration behind
// those answers is one the document never describes.
//
// So the check runs BEFORE any mutation and names what it refused. It is the
// C++ twin of the Rust runtime's `helpers::configuration::validate`, asking
// the same questions of the same static hierarchy, so a configuration one
// engine accepts is one the other accepts.

#pragma once

#include <algorithm>
#include <cstddef>
#include <optional>
#include <vector>

namespace SCE::Core {

/// `a == b` for two NUL-terminated strings, usable in a constant expression.
///
/// `std::strcmp` is constexpr only as a compiler extension, and the generated
/// `getStateName` / `getStateFromName` round trip is asserted at COMPILE time —
/// so it needs a comparison the standard guarantees is available there.
[[nodiscard]] constexpr bool constexprStrEq(const char *a, const char *b) noexcept {
    if (a == nullptr || b == nullptr) {
        return a == b;
    }
    while (*a != '\0' && *a == *b) {
        ++a;
        ++b;
    }
    return *a == *b;
}

/// Why a configuration was refused. `None` is the accepting answer.
///
/// An enumerated reason rather than a bool: a host handing back a journal it
/// wrote itself needs to know WHICH rule its record broke, and "invalid" sends
/// it looking at the wrong half.
enum class ConfigurationRejection {
    None,
    /// No states at all. A machine is never in nothing.
    Empty,
    /// A state appears twice. Checked first, because every arity count below
    /// would otherwise read a duplicate as a second child and blame the wrong
    /// rule.
    Duplicate,
    /// A state is present whose parent is not — the set is not ancestor-closed.
    AncestorMissing,
    /// §scxml-3.11: a configuration closes on exactly one root.
    RootCount,
    /// §scxml-3.11: a compound state holds exactly one active child.
    CompoundChildCount,
    /// §scxml-3.11: a `<parallel>` holds EVERY region, and one is missing.
    ParallelRegionMissing,
    /// §scxml-3.11: a `<parallel>` holds every region and nothing else.
    ParallelChildCount,
    /// An atomic state has a child in the set, so it is not atomic here.
    AtomicHasChildren,
    /// The current state is not in the configuration it is supposed to be in.
    CurrentNotActive,
    /// The current state is compound or parallel. §scxml-3.11 makes the current
    /// state the atomic one the engine descended to.
    CurrentNotAtomic,
};

/// A human-readable reason, for the message a refusal carries.
[[nodiscard]] constexpr const char *configurationRejectionText(ConfigurationRejection r) noexcept {
    switch (r) {
    case ConfigurationRejection::None:
        return "accepted";
    case ConfigurationRejection::Empty:
        return "the configuration is empty; a machine is never in nothing";
    case ConfigurationRejection::Duplicate:
        return "a state appears twice";
    case ConfigurationRejection::AncestorMissing:
        return "a state is present whose parent is not, so the set is not ancestor-closed";
    case ConfigurationRejection::RootCount:
        return "a configuration closes on exactly one root (W3C SCXML 3.11)";
    case ConfigurationRejection::CompoundChildCount:
        return "a compound state holds exactly one active child (W3C SCXML 3.11)";
    case ConfigurationRejection::ParallelRegionMissing:
        return "a <parallel> holds every region and one is missing (W3C SCXML 3.11)";
    case ConfigurationRejection::ParallelChildCount:
        return "a <parallel> holds every region and nothing else (W3C SCXML 3.11)";
    case ConfigurationRejection::AtomicHasChildren:
        return "an atomic state has a child in the set";
    case ConfigurationRejection::CurrentNotActive:
        return "the current state is not in the configuration";
    case ConfigurationRejection::CurrentNotAtomic:
        return "the current state must be the atomic state the engine descended to";
    }
    return "unknown";
}

/// Whether `configuration` is a configuration of `Policy`'s document, with
/// `current` as its current state.
///
/// Pure: reads the policy's static hierarchy and nothing else. Cost is
/// quadratic in the chain length, which is a handful of states, and this runs
/// once per restore — the shape is chosen for being obviously right rather
/// than for being fast, exactly as its Rust twin is.
template <typename Policy>
[[nodiscard]] ConfigurationRejection validateConfiguration(const std::vector<typename Policy::State> &configuration,
                                                           typename Policy::State current) {
    using State = typename Policy::State;

    if (configuration.empty()) {
        return ConfigurationRejection::Empty;
    }

    for (std::size_t i = 0; i < configuration.size(); ++i) {
        for (std::size_t j = 0; j < i; ++j) {
            if (configuration[j] == configuration[i]) {
                return ConfigurationRejection::Duplicate;
            }
        }
    }

    const auto holds = [&configuration](State s) {
        for (const State &member : configuration) {
            if (member == s) {
                return true;
            }
        }
        return false;
    };

    std::size_t roots = 0;
    for (const State &state : configuration) {
        const std::optional<State> parent = Policy::getParent(state);
        if (!parent.has_value()) {
            ++roots;
        } else if (!holds(*parent)) {
            return ConfigurationRejection::AncestorMissing;
        }
    }
    if (roots != 1) {
        return ConfigurationRejection::RootCount;
    }

    for (const State &state : configuration) {
        std::size_t children = 0;
        for (const State &candidate : configuration) {
            const std::optional<State> parent = Policy::getParent(candidate);
            if (parent.has_value() && *parent == state) {
                ++children;
            }
        }

        if constexpr (Policy::HAS_PARALLEL_STATES) {
            if (Policy::isParallelState(state)) {
                const std::vector<State> regions = Policy::getParallelRegions(state);
                for (const State &region : regions) {
                    if (!holds(region)) {
                        return ConfigurationRejection::ParallelRegionMissing;
                    }
                }
                if (children != regions.size()) {
                    return ConfigurationRejection::ParallelChildCount;
                }
                continue;
            }
        }

        if (Policy::isCompoundState(state)) {
            if (children != 1) {
                return ConfigurationRejection::CompoundChildCount;
            }
        } else if (children != 0) {
            return ConfigurationRejection::AtomicHasChildren;
        }
    }

    if (!holds(current)) {
        return ConfigurationRejection::CurrentNotActive;
    }
    if (Policy::isCompoundState(current)) {
        return ConfigurationRejection::CurrentNotAtomic;
    }
    if constexpr (Policy::HAS_PARALLEL_STATES) {
        if (Policy::isParallelState(current)) {
            return ConfigurationRejection::CurrentNotAtomic;
        }
    }

    return ConfigurationRejection::None;
}

/// §scxml-3.11: which rule a state SPECIFICATION breaks. A specification is
/// what a transition's `target`, a state's `initial`, or a `<history>`
/// default names — states, without their ancestors or default descendants —
/// and it is legal when (1) no state on it is an ancestor of another and (2)
/// adding all ancestors and default descendants yields a legal configuration.
enum class SpecificationBreach {
    None,
    /// Rule 1: one state on the list is an ancestor of another.
    Ancestor,
    /// Rule 2: two states meet at a state that is not a `<parallel>` — a
    /// compound state, or the `<scxml>` element — which holds exactly one
    /// child in any configuration, so both cannot be active.
    NotParallel,
    /// An `initial` value or `<history>` default names a state outside the
    /// state that holds it.
    OutsideContainer,
};

/// A breach and the states it is about. `first` / `second` read per breach:
/// `Ancestor` — the ancestor, then its descendant; `NotParallel` — the two
/// states, with `meet` the state they meet at (`nullopt` = `<scxml>`);
/// `OutsideContainer` — the state outside, then the container.
template <typename State> struct SpecificationVerdict {
    SpecificationBreach breach = SpecificationBreach::None;
    State first{};
    State second{};
    std::optional<State> meet;
};

/// Whether `states` is a legal state specification — the rules
/// `SpecificationBreach` names.
///
/// `parentOf(s)` answers the state `s` sits directly under, `nullopt` at the
/// top of the document; a `<history>` answers the state it is declared in.
/// `isParallel(s)` answers whether `s` is a `<parallel>`. `container` is the
/// state an `initial` value or `<history>` default is restricted to the
/// descendants of, and `nullopt` for a transition target.
///
/// Rule 2 is asked pairwise: adding two states' ancestors adds the state they
/// meet at, and unless that is a `<parallel>` it holds only one of them. A
/// state named twice is not a breach; the list is a set.
///
/// Lambda-injected so the Interpreter (string ids) and the generated code
/// (enum states) ask one definition — the Rust generator's
/// `scxml_references::specification_breach` is its twin, and the two must
/// accept and refuse the same documents.
template <typename State, typename ParentOf, typename IsParallel>
[[nodiscard]] SpecificationVerdict<State> checkStateSpecification(const std::vector<State> &states,
                                                                  const std::optional<State> &container,
                                                                  ParentOf parentOf, IsParallel isParallel) {
    const auto properAncestors = [&parentOf](const State &s) {
        std::vector<State> out;
        for (std::optional<State> p = parentOf(s); p.has_value(); p = parentOf(*p)) {
            out.push_back(*p);
        }
        return out;
    };
    const auto contains = [](const std::vector<State> &list, const State &s) {
        return std::find(list.begin(), list.end(), s) != list.end();
    };

    if (container.has_value()) {
        for (const State &s : states) {
            if (!contains(properAncestors(s), *container)) {
                return {SpecificationBreach::OutsideContainer, s, *container, std::nullopt};
            }
        }
    }
    for (std::size_t i = 0; i < states.size(); ++i) {
        for (std::size_t j = i + 1; j < states.size(); ++j) {
            const State &first = states[i];
            const State &second = states[j];
            if (first == second) {
                continue;
            }
            const std::vector<State> aboveFirst = properAncestors(first);
            const std::vector<State> aboveSecond = properAncestors(second);
            if (contains(aboveSecond, first)) {
                return {SpecificationBreach::Ancestor, first, second, std::nullopt};
            }
            if (contains(aboveFirst, second)) {
                return {SpecificationBreach::Ancestor, second, first, std::nullopt};
            }
            std::optional<State> meet;
            for (const State &a : aboveFirst) {
                if (contains(aboveSecond, a)) {
                    meet = a;
                    break;
                }
            }
            if (!meet.has_value() || !isParallel(*meet)) {
                return {SpecificationBreach::NotParallel, first, second, meet};
            }
        }
    }
    return {};
}

}  // namespace SCE::Core
