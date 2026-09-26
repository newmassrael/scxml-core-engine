// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#if __cpp_concepts >= 202002L
#include <concepts>
#endif

#include "common/ForwardedEvent.h"

#include <optional>
#include <string>
#include <type_traits>
#include <vector>

namespace SCE::Core {

// ═══════════════════════════════════════════════════════════════════════════════
// C++17-compatible feature detection traits
//
// These type traits work in both C++17 and C++20.
// Used in if constexpr for optional policy feature detection.
// ═══════════════════════════════════════════════════════════════════════════════

template <typename P, typename E, typename = void> struct HasDataModelInitTrait : std::false_type {};

template <typename P, typename E>
struct HasDataModelInitTrait<P, E, std::void_t<decltype(std::declval<P>().initializeDataModel(std::declval<E &>()))>>
    : std::true_type {};

template <typename P, typename E, typename = void> struct HasInvokeSupportTrait : std::false_type {};

template <typename P, typename E>
struct HasInvokeSupportTrait<P, E, std::void_t<decltype(std::declval<P>().executePendingInvokes(std::declval<E &>()))>>
    : std::true_type {};

template <typename P, typename E, typename = void> struct HasChildTickTrait : std::false_type {};

template <typename P, typename E>
struct HasChildTickTrait<P, E, std::void_t<decltype(std::declval<P>().tickChildren(std::declval<E &>()))>>
    : std::true_type {};

template <typename P, typename E, typename = void> struct HasAutoforwardTrait : std::false_type {};

template <typename P, typename E>
struct HasAutoforwardTrait<P, E,
                           std::void_t<decltype(std::declval<P>().forwardToAutoforwardChildren(
                               std::declval<const ::SCE::Common::ForwardedEvent &>(), std::declval<E &>()))>>
    : std::true_type {};

template <typename P, typename M, typename E, typename = void> struct HasFinalizeTrait : std::false_type {};

template <typename P, typename M, typename E>
struct HasFinalizeTrait<P, M, E,
                        std::void_t<decltype(std::declval<P>().executeFinalizeForChildEvent(
                            std::declval<const M &>(), std::declval<E &>()))>> : std::true_type {};

template <typename P, typename = void> struct HasActiveStatesTrait : std::false_type {};

template <typename P>
struct HasActiveStatesTrait<P, std::void_t<decltype(std::declval<P>().getActiveStates())>> : std::true_type {};

/// Policy declares that driving it needs `tick()` rather than `step()` alone.
///
/// `step()` runs a macrostep and never drains the delayed-send scheduler or
/// ticks an invoked child, so a host that only ever calls it gets no delayed
/// event, no error and no warning. The generator knows which machines are in
/// that position and now says so on the policy; this detects the answer.
///
/// Detected rather than required so a hand-written policy — the mesh worker
/// harnesses, and any consumer's own — keeps compiling. Absent means `false`,
/// which is what a policy with no delayed send would have declared anyway.
template <typename P, typename = void> struct NeedsEventSchedulerTrait : std::false_type {};

template <typename P>
struct NeedsEventSchedulerTrait<P, std::void_t<decltype(P::NEEDS_EVENT_SCHEDULER)>>
    : std::bool_constant<P::NEEDS_EVENT_SCHEDULER> {};

/// Policy names the `<invoke>`s its document hands to a host invoker
/// (§scxml-6.4.1), as `static constexpr` `HOST_INVOKE_IDS` — a range of
/// strings. Their `done.invoke.<id>` is accepted only through the engine's
/// `completeHostInvoke`. Detected rather than required: a policy without a
/// host-run invoke has nothing to list, and a hand-written one keeps compiling.
template <typename P, typename = void> struct HasHostInvokeIdsTrait : std::false_type {};

template <typename P> struct HasHostInvokeIdsTrait<P, std::void_t<decltype(P::HOST_INVOKE_IDS)>> : std::true_type {};

/// Policy can hand an event to a live `<invoke>` child named by session id.
///
/// A session's published location is a usable `<send>` target,
/// so an event addressed to a child has to reach that child rather than the
/// sender's own queue. Only a policy with local scxml invokes owns child
/// instances to route to, which is why this is detected rather than required.
template <typename P, typename = void> struct HasChildSessionDeliveryTrait : std::false_type {};

template <typename P>
struct HasChildSessionDeliveryTrait<
    P, std::void_t<decltype(std::declval<P &>().deliverToChildSession(
           std::declval<const std::string &>(), std::declval<const SCE::Common::ForwardedEvent &>()))>>
    : std::true_type {};

// ═══════════════════════════════════════════════════════════════════════════════
// C++20 concepts + C++17 constexpr bool aliases
//
// When __cpp_concepts is available (C++20): full concept definitions for template
// constraints and clear error messages, plus concept aliases for feature detection.
// When C++17: constexpr bool aliases from traits for if constexpr usage.
//
// The if constexpr usage pattern works identically in both modes:
//   if constexpr (HasDataModelInit<P, Engine>) { ... }
// C++20 concept converts to bool; C++17 constexpr bool is already bool.
// ═══════════════════════════════════════════════════════════════════════════════

#if __cpp_concepts >= 202002L

// ─────────────────────────────────────────────────────────────────────────────
// StatePolicy Concept Hierarchy
//
// Formalizes the implicit interface contract between StaticExecutionEngine,
// helper classes, and generated StatePolicy structs.
//
// Layered design — each level adds requirements incrementally:
//   HierarchyPolicy     → the document tree Appendix D walks (parents,
//                         children in document order, compound / parallel)
//   BaseStatePolicy     → engine essentials (initial and history targets,
//                         isFinalState, enums)
//   EventNamingPolicy   → event name ↔ enum conversion
//   StateNamingPolicy   → state enum → SCXML id string
//   ParallelStatePolicy → parallel state extensions (isParallelState, regions)
//
// NOTE: Engine-dependent checks (firstEnabledTransition, executeEntryActions,
// etc.) are intentionally NOT in concepts due to circular dependency:
//   StaticExecutionEngine<P> requires P, but P's methods take Engine& parameter.
// The engine's calls to them are what checks them.
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Level 1: Hierarchy Policy — the document tree
// Used by: HierarchicalStateHelper, ParallelTransitionHelper,
//          ConflictResolutionHelper, StaticExecutionEngine
// ─────────────────────────────────────────────────────────────────────────────

/// The state tree the Appendix D procedures walk: parents, children in
/// document order, and which states are compound or parallel.
/// `isCompoundState` answers true for a `<state>` with child states; what it
/// answers for a `<parallel>` is not part of the contract — policies differ —
/// so a caller that must tell the two apart asks `isParallelState` as well.
template <typename P>
concept HierarchyPolicy = requires { typename P::State; } && requires(typename P::State s) {
    { P::getParent(s) } -> std::same_as<std::optional<typename P::State>>;
    { P::isCompoundState(s) } -> std::convertible_to<bool>;
    { P::isParallelState(s) } -> std::convertible_to<bool>;
    { P::getChildStates(s) } -> std::convertible_to<std::vector<typename P::State>>;
    { P::getDocumentOrder(s) } -> std::convertible_to<int>;
};

// ─────────────────────────────────────────────────────────────────────────────
// Level 2: Base State Policy — core engine requirements
// Used by: StaticExecutionEngine (template constraint)
// ─────────────────────────────────────────────────────────────────────────────

/// Core state identification and lifecycle capability, and the target lists
/// the entry procedures dereference: every compound state's initial
/// transition, the document's, and every `<history>`'s default.
/// Every StatePolicy used with StaticExecutionEngine must satisfy this.
template <typename P>
concept BaseStatePolicy = HierarchyPolicy<P> && requires {
    typename P::Event;
    typename P::History;
    { P::HAS_PARALLEL_STATES } -> std::convertible_to<bool>;
    P::getDocumentInitialTargets();
} && requires(typename P::State s, typename P::History h) {
    { P::initialState() } -> std::same_as<typename P::State>;
    { P::isFinalState(s) } -> std::convertible_to<bool>;
    P::getInitialTargets(s);
    { P::getHistoryParent(h) } -> std::same_as<typename P::State>;
    P::getHistoryDefaultTargets(h);
};

// ─────────────────────────────────────────────────────────────────────────────
// Level 3: Event Naming Policy — event name ↔ enum conversion
// Used by: StaticExecutionEngine (raiseExternal by name, HTTP sends)
// ─────────────────────────────────────────────────────────────────────────────

/// Event name resolution capability.
/// Required for any policy that handles external events or HTTP communication.
template <typename P>
concept EventNamingPolicy = BaseStatePolicy<P> && requires(P p, typename P::Event e, const std::string &name) {
    { p.getEventName(e) } -> std::convertible_to<std::string>;
    { p.getEventFromName(name) } -> std::same_as<std::optional<typename P::Event>>;
};

// ─────────────────────────────────────────────────────────────────────────────
// Level 3.5: State Naming Policy — state enum → SCXML id string
// Used by: InPredicateHelper, trace recorders, post-mortem analyzers
// ─────────────────────────────────────────────────────────────────────────────

/// State id string resolution capability.
/// The mapping is structural (independent of parallel states / In() predicate),
/// so every generated policy must provide it. Mirrors EventNamingPolicy's
/// shape — the unconditional `getStateName` emit in utility_methods.jinja2
/// satisfies this contract.
template <typename P>
concept StateNamingPolicy = BaseStatePolicy<P> && requires(typename P::State s) {
    { P::getStateName(s) } -> std::convertible_to<const char *>;
};

// ─────────────────────────────────────────────────────────────────────────────
// Level 4: Parallel State Policy — parallel state extensions
// Used by: ParallelStateHelper, ParallelCompletionHelper
// ─────────────────────────────────────────────────────────────────────────────

/// Parallel state navigation and ordering capability.
/// Required when HAS_PARALLEL_STATES is true.
template <typename P>
concept ParallelStatePolicy = HierarchyPolicy<P> && requires(typename P::State s1, typename P::State s2) {
    { P::isParallelState(s1) } -> std::convertible_to<bool>;
    { P::getParallelRegions(s1) } -> std::convertible_to<std::vector<typename P::State>>;
    { P::isDescendantOf(s1, s2) } -> std::convertible_to<bool>;
    { P::getDocumentOrder(s1) } -> std::convertible_to<int>;
    { P::isFinalState(s1) } -> std::convertible_to<bool>;
};

// ─────────────────────────────────────────────────────────────────────────────
// Optional Feature Detection — C++20 concept aliases
//
// These replace anonymous `if constexpr (requires { ... })` blocks with
// named concepts for readability. The if constexpr pattern is preserved —
// these concepts are used as named conditions, not as template constraints.
// ─────────────────────────────────────────────────────────────────────────────

/// Policy supports datamodel initialization (JSEngine-backed state machines)
template <typename P, typename Engine>
concept HasDataModelInit = HasDataModelInitTrait<P, Engine>::value;

/// Policy supports invoke lifecycle (static or hybrid invokes)
template <typename P, typename Engine>
concept HasInvokeSupport = HasInvokeSupportTrait<P, Engine>::value;

/// Policy supports child state machine ticking
template <typename P, typename Engine>
concept HasChildTick = HasChildTickTrait<P, Engine>::value;

/// Policy supports autoforward to children (§scxml-6.4)
template <typename P, typename Engine>
concept HasAutoforward = HasAutoforwardTrait<P, Engine>::value;

/// Policy supports finalize handler execution (§scxml-6.5)
template <typename P, typename EventMeta, typename Engine>
concept HasFinalize = HasFinalizeTrait<P, EventMeta, Engine>::value;

/// Policy tracks active states across parallel regions
template <typename P>
concept HasActiveStates = HasActiveStatesTrait<P>::value;

/// Policy can deliver an event to a live invoke child by session id
template <typename P>
concept HasChildSessionDelivery = HasChildSessionDeliveryTrait<P>::value;

/// Driving the policy's machine needs `tick()`, not `step()` alone
template <typename P>
concept NeedsEventScheduler = NeedsEventSchedulerTrait<P>::value;

/// Policy lists the invokes a host runs
template <typename P>
concept HasHostInvokeIds = HasHostInvokeIdsTrait<P>::value;

#else  // C++17 fallback

// ─────────────────────────────────────────────────────────────────────────────
// C++17: constexpr bool aliases for if constexpr usage
//
// No concept definitions — template constraints use plain typename.
// Feature detection works identically: if constexpr (HasDataModelInit<P, E>)
// ─────────────────────────────────────────────────────────────────────────────

template <typename P, typename Engine> inline constexpr bool HasDataModelInit = HasDataModelInitTrait<P, Engine>::value;

template <typename P, typename Engine> inline constexpr bool HasInvokeSupport = HasInvokeSupportTrait<P, Engine>::value;

template <typename P, typename Engine> inline constexpr bool HasChildTick = HasChildTickTrait<P, Engine>::value;

template <typename P, typename Engine> inline constexpr bool HasAutoforward = HasAutoforwardTrait<P, Engine>::value;

template <typename P, typename M, typename Engine>
inline constexpr bool HasFinalize = HasFinalizeTrait<P, M, Engine>::value;

template <typename P> inline constexpr bool HasActiveStates = HasActiveStatesTrait<P>::value;

template <typename P> inline constexpr bool HasChildSessionDelivery = HasChildSessionDeliveryTrait<P>::value;

template <typename P> inline constexpr bool NeedsEventScheduler = NeedsEventSchedulerTrait<P>::value;

template <typename P> inline constexpr bool HasHostInvokeIds = HasHostInvokeIdsTrait<P>::value;

#endif  // __cpp_concepts >= 202002L

}  // namespace SCE::Core
