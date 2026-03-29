// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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

#include <concepts>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Core {

// ═══════════════════════════════════════════════════════════════════════════════
// StatePolicy Concept Hierarchy
//
// Formalizes the implicit interface contract between StaticExecutionEngine,
// helper classes, and generated StatePolicy structs.
//
// Layered design — each level adds requirements incrementally:
//   HierarchyPolicy     → basic tree traversal (getParent, isCompoundState)
//   BaseStatePolicy     → engine essentials (initialState, isFinalState, enums)
//   EventNamingPolicy   → event name ↔ enum conversion
//   ParallelStatePolicy → parallel state extensions (isParallelState, regions)
//
// NOTE: Engine-dependent checks (processTransition, executeEntryActions, etc.)
// are intentionally NOT in concepts due to circular dependency:
//   StaticExecutionEngine<P> requires P, but P's methods take Engine& parameter.
// These are verified via static_assert inside StaticExecutionEngine instead.
//
// NOTE: Member variable checks (lastTransitionIsInternal_, etc.) are also
// verified via static_assert because concepts cannot distinguish between
// member variables and member functions with the same syntax when accessed
// through friend declarations.
// ═══════════════════════════════════════════════════════════════════════════════

// ─────────────────────────────────────────────────────────────────────────────
// Level 1: Hierarchy Policy — minimal tree traversal
// Used by: HierarchicalStateHelper, HierarchicalAlgorithms
// ─────────────────────────────────────────────────────────────────────────────

/// Minimal state hierarchy navigation capability.
/// Any policy used with HierarchicalStateHelper must satisfy this.
template <typename P>
concept HierarchyPolicy = requires {
    typename P::State;
} && requires(typename P::State s) {
    { P::getParent(s) } -> std::same_as<std::optional<typename P::State>>;
    { P::isCompoundState(s) } -> std::convertible_to<bool>;
    { P::getInitialChild(s) } -> std::same_as<typename P::State>;
};

// ─────────────────────────────────────────────────────────────────────────────
// Level 2: Base State Policy — core engine requirements
// Used by: StaticExecutionEngine (template constraint)
// ─────────────────────────────────────────────────────────────────────────────

/// Core state identification and lifecycle capability.
/// Every StatePolicy used with StaticExecutionEngine must satisfy this.
template <typename P>
concept BaseStatePolicy = HierarchyPolicy<P> && requires {
    typename P::Event;
    { P::HAS_PARALLEL_STATES } -> std::convertible_to<bool>;
} && requires(typename P::State s) {
    { P::initialState() } -> std::same_as<typename P::State>;
    { P::isFinalState(s) } -> std::convertible_to<bool>;
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
// Level 4: Parallel State Policy — parallel state extensions
// Used by: ParallelTransitionHelper, ParallelStateHelper,
//          ParallelCompletionHelper, ParallelExitEntryHelper,
//          ConflictResolutionHelper
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
// Optional Feature Detection Concepts
//
// These replace anonymous `if constexpr (requires { ... })` blocks with
// named concepts for readability. The if constexpr pattern is preserved —
// these concepts are used as named conditions, not as template constraints.
// ─────────────────────────────────────────────────────────────────────────────

/// Policy supports datamodel initialization (JSEngine-backed state machines)
template <typename P, typename Engine>
concept HasDataModelInit = requires(P p, Engine &eng) {
    { p.initializeDataModel(eng) };
};

/// Policy supports invoke lifecycle (static or hybrid invokes)
template <typename P, typename Engine>
concept HasInvokeSupport = requires(P p, Engine &eng) {
    { p.executePendingInvokes(eng) };
};

/// Policy supports child state machine ticking
template <typename P, typename Engine>
concept HasChildTick = requires(P p, Engine &eng) {
    { p.tickChildren(eng) };
};

/// Policy supports autoforward to children (W3C SCXML 6.4.6)
template <typename P, typename Engine>
concept HasAutoforward = requires(P p, const std::string &name, Engine &eng) {
    { p.forwardToAutoforwardChildren(name, eng) };
};

/// Policy supports finalize handler execution (W3C SCXML 6.5)
template <typename P, typename EventMeta, typename Engine>
concept HasFinalize = requires(P p, const EventMeta &meta, Engine &eng) {
    { p.executeFinalizeForChildEvent(meta, eng) };
};

/// Policy tracks active states across parallel regions
template <typename P>
concept HasActiveStates = requires(P p) {
    { p.getActiveStates() } -> std::convertible_to<std::vector<typename P::State>>;
};

/// Policy has external event marking flag
template <typename P>
concept HasExternalEventFlag = requires(P p) {
    { p.nextEventIsExternal_ } -> std::convertible_to<bool>;
};

}  // namespace SCE::Core
