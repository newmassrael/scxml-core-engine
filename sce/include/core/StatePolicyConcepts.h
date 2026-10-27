// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "common/ForwardedEvent.h"

#include <optional>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

// A consumer compiles this header at C++17 and SCE compiles its own sources at
// C++20, into one program (docs/adr/0059). Every contract here is therefore a
// C++17 trait, which means the same thing in both: a definition that changed
// with the language standard would break the One Definition Rule.

namespace SCE::Core {

// ═══════════════════════════════════════════════════════════════════════════════
// Optional feature detection traits
//
// Asked through `::value` in `if constexpr` for optional policy features.
// ═══════════════════════════════════════════════════════════════════════════════

/// Policy supports datamodel initialization (JSEngine-backed state machines)
template <typename P, typename E, typename = void> struct HasDataModelInitTrait : std::false_type {};

template <typename P, typename E>
struct HasDataModelInitTrait<P, E, std::void_t<decltype(std::declval<P>().initializeDataModel(std::declval<E &>()))>>
    : std::true_type {};

/// Policy supports invoke lifecycle (static or hybrid invokes)
template <typename P, typename E, typename = void> struct HasInvokeSupportTrait : std::false_type {};

template <typename P, typename E>
struct HasInvokeSupportTrait<P, E, std::void_t<decltype(std::declval<P>().executePendingInvokes(std::declval<E &>()))>>
    : std::true_type {};

/// Policy supports child state machine ticking
template <typename P, typename E, typename = void> struct HasChildTickTrait : std::false_type {};

template <typename P, typename E>
struct HasChildTickTrait<P, E, std::void_t<decltype(std::declval<P>().tickChildren(std::declval<E &>()))>>
    : std::true_type {};

// §scxml-6.4: a policy that names the nearest deadline of the children it runs.
// Emitted beside `tickChildren`, so a policy that ticks children answers this
// too; the engine asks it only through this trait, never by probing the member
// directly (a member that sits under another guard of the template would read
// as absent and the branch would silently go the other way).
template <typename P, typename = void> struct HasNextChildDeadlineTrait : std::false_type {};

template <typename P>
struct HasNextChildDeadlineTrait<P, std::void_t<decltype(std::declval<const P &>().nextChildDeadline())>>
    : std::true_type {};

// Child sessions in Mesh (SCE_MESH.md): a policy that tells the Mesh routers of
// the children it runs that their sessions ended. Emitted beside `tickChildren`,
// and asked only through this trait, for the reason the one above gives.
template <typename P, typename = void> struct HasChildMeshDetachTrait : std::false_type {};

template <typename P>
struct HasChildMeshDetachTrait<P, std::void_t<decltype(std::declval<P &>().detachChildMeshSessions())>>
    : std::true_type {};

/// Policy supports autoforward to children (§scxml-6.4)
template <typename P, typename E, typename = void> struct HasAutoforwardTrait : std::false_type {};

template <typename P, typename E>
struct HasAutoforwardTrait<P, E,
                           std::void_t<decltype(std::declval<P>().forwardToAutoforwardChildren(
                               std::declval<const ::SCE::Common::ForwardedEvent &>(), std::declval<E &>()))>>
    : std::true_type {};

/// Policy supports finalize handler execution (§scxml-6.5)
template <typename P, typename M, typename E, typename = void> struct HasFinalizeTrait : std::false_type {};

template <typename P, typename M, typename E>
struct HasFinalizeTrait<P, M, E,
                        std::void_t<decltype(std::declval<P>().executeFinalizeForChildEvent(
                            std::declval<const M &>(), std::declval<E &>()))>> : std::true_type {};

/// Policy tracks active states across parallel regions
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

/// Policy declares that its document sends to its parent session — a literal
/// `<send target="#_parent">` — the generate manifest's `needs_parent`.
///
/// Read by a host that refuses to start such a machine as a root
/// (`StaticExecutionEngine::initializeAsRoot`). Detected rather than required,
/// for the reason `NeedsEventSchedulerTrait` gives; absent means `false`.
template <typename P, typename = void> struct NeedsParentTrait : std::false_type {};

template <typename P>
struct NeedsParentTrait<P, std::void_t<decltype(P::NEEDS_PARENT)>> : std::bool_constant<P::NEEDS_PARENT> {};

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

/// §scxml-6.2 + 6.4: policy can deliver a delayed send's event to one of
/// its invocations by invoke id, answering whether that invocation was there
/// to take it. Only a policy that invokes has children to deliver to.
template <typename P, typename = void> struct HasInvocationDeliveryTrait : std::false_type {};

template <typename P>
struct HasInvocationDeliveryTrait<
    P, std::void_t<decltype(std::declval<P &>().deliverToInvocation(
           std::declval<const std::string &>(), std::declval<const SCE::Common::ForwardedEvent &>()))>>
    : std::true_type {};

/// §scxml-6.2 + 6.4: policy can deliver a delayed send's event to the
/// session that invoked it, answering whether that session was there. Only an
/// invoked machine has a parent.
template <typename P, typename = void> struct HasParentDeliveryTrait : std::false_type {};

template <typename P>
struct HasParentDeliveryTrait<
    P, std::void_t<decltype(std::declval<P &>().deliverToParent(std::declval<const SCE::Common::ForwardedEvent &>()))>>
    : std::true_type {};

/// The id of the session this policy is, as text (`_sessionid`; docs/adr/0045): a
/// request to a host invoker names the session that made it, and a saved state names
/// the session it holds. Emitted on a policy that holds a session id; a machine with
/// neither a script engine, an `<invoke>` nor a saved state has none to give, and its
/// requests carry an empty one.
template <typename P, typename = void> struct HasSessionIdTextTrait : std::false_type {};

template <typename P>
struct HasSessionIdTextTrait<P, std::void_t<decltype(std::declval<const P &>().sessionIdText())>> : std::true_type {};

/// The policy runs child sessions that a host's invokers serve (docs/adr/0045): it
/// hands the tree's registry to each, and finds the session a host-run invocation's
/// ending is for among them. `Ending` is `HostInvokeEnding`, a parameter so this header
/// need not include the one that declares it. Emitted beside `tickChildren`, so a policy
/// that ticks children answers it too, and asked only through this trait.
template <typename P, typename Ending, typename = void> struct HasHostInvokerChildrenTrait : std::false_type {};

template <typename P, typename Ending>
struct HasHostInvokerChildrenTrait<P, Ending,
                                   std::void_t<decltype(std::declval<P &>().routeHostInvokeEnd(
                                       std::declval<const std::string &>(), std::declval<const Ending &>()))>>
    : std::true_type {};

// ═══════════════════════════════════════════════════════════════════════════════
// StatePolicy contract traits
//
// Formalizes the implicit interface contract between StaticExecutionEngine,
// helper classes, and generated StatePolicy structs. A template that needs a
// level checks it with `static_assert(<Level>Trait<P>::value, ...)` as the first
// line of its body.
//
// Layered design — each level adds requirements incrementally:
//   HierarchyPolicyTrait     → the document tree Appendix D walks (parents,
//                              children in document order, compound / parallel)
//   BaseStatePolicyTrait     → engine essentials (initial and history targets,
//                              isFinalState, enums)
//   EventNamingPolicyTrait   → event name ↔ enum conversion
//   StateNamingPolicyTrait   → state enum → SCXML id string
//   ParallelStatePolicyTrait → parallel state extensions (isParallelState, regions)
//
// Each trait detects its expressions in `std::void_t` and then checks their
// types: `std::is_same` where the result must be exactly that type, and
// `std::is_convertible` where it must convert to it. An expression is asked of
// an lvalue of its argument type, as a requires-expression parameter would be.
//
// NOTE: Engine-dependent checks (firstEnabledTransition, executeEntryActions,
// etc.) are intentionally NOT in these traits due to circular dependency:
//   StaticExecutionEngine<P> requires P, but P's methods take Engine& parameter.
// The engine's calls to them are what checks them.
// ═══════════════════════════════════════════════════════════════════════════════

namespace detail {

// The expressions the contract traits ask of a policy, each named once so a
// trait's detection list and its type checks name the same expression.
template <typename P> using PolicyStateLvalue = typename P::State &;
template <typename P> using PolicyHistoryLvalue = typename P::History &;

template <typename P> using GetParentResult = decltype(P::getParent(std::declval<PolicyStateLvalue<P>>()));
template <typename P> using IsCompoundStateResult = decltype(P::isCompoundState(std::declval<PolicyStateLvalue<P>>()));
template <typename P> using IsParallelStateResult = decltype(P::isParallelState(std::declval<PolicyStateLvalue<P>>()));
template <typename P> using GetChildStatesResult = decltype(P::getChildStates(std::declval<PolicyStateLvalue<P>>()));
template <typename P>
using GetDocumentOrderResult = decltype(P::getDocumentOrder(std::declval<PolicyStateLvalue<P>>()));
template <typename P> using IsFinalStateResult = decltype(P::isFinalState(std::declval<PolicyStateLvalue<P>>()));
template <typename P>
using GetParallelRegionsResult = decltype(P::getParallelRegions(std::declval<PolicyStateLvalue<P>>()));
template <typename P>
using IsDescendantOfResult =
    decltype(P::isDescendantOf(std::declval<PolicyStateLvalue<P>>(), std::declval<PolicyStateLvalue<P>>()));

template <typename P> using HasParallelStatesResult = decltype((P::HAS_PARALLEL_STATES));
template <typename P> using GetDocumentInitialTargetsResult = decltype(P::getDocumentInitialTargets());
template <typename P> using InitialStateResult = decltype(P::initialState());
template <typename P>
using GetInitialTargetsResult = decltype(P::getInitialTargets(std::declval<PolicyStateLvalue<P>>()));
template <typename P>
using GetHistoryParentResult = decltype(P::getHistoryParent(std::declval<PolicyHistoryLvalue<P>>()));
template <typename P>
using GetHistoryDefaultTargetsResult = decltype(P::getHistoryDefaultTargets(std::declval<PolicyHistoryLvalue<P>>()));

template <typename P>
using GetEventNameResult = decltype(std::declval<P &>().getEventName(std::declval<typename P::Event &>()));
template <typename P>
using GetEventFromNameResult = decltype(std::declval<P &>().getEventFromName(std::declval<const std::string &>()));

template <typename P> using GetStateNameResult = decltype(P::getStateName(std::declval<PolicyStateLvalue<P>>()));

}  // namespace detail

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
///
/// Requires `P::State`, and for a `State s`: `P::getParent(s)` returning
/// exactly `std::optional<State>`; `P::isCompoundState(s)` and
/// `P::isParallelState(s)` convertible to `bool`; `P::getChildStates(s)`
/// convertible to `std::vector<State>`; `P::getDocumentOrder(s)` convertible
/// to `int`.
template <typename P, typename = void> struct HierarchyPolicyTrait : std::false_type {};

template <typename P>
struct HierarchyPolicyTrait<P, std::void_t<typename P::State, detail::GetParentResult<P>,
                                           detail::IsCompoundStateResult<P>, detail::IsParallelStateResult<P>,
                                           detail::GetChildStatesResult<P>, detail::GetDocumentOrderResult<P>>>
    : std::bool_constant<std::is_same_v<detail::GetParentResult<P>, std::optional<typename P::State>> &&
                         std::is_convertible_v<detail::IsCompoundStateResult<P>, bool> &&
                         std::is_convertible_v<detail::IsParallelStateResult<P>, bool> &&
                         std::is_convertible_v<detail::GetChildStatesResult<P>, std::vector<typename P::State>> &&
                         std::is_convertible_v<detail::GetDocumentOrderResult<P>, int>> {};

// ─────────────────────────────────────────────────────────────────────────────
// Level 2: Base State Policy — core engine requirements
// Used by: StaticExecutionEngine, through EventNamingPolicyTrait
// ─────────────────────────────────────────────────────────────────────────────

/// Core state identification and lifecycle capability, and the target lists
/// the entry procedures dereference: every compound state's initial
/// transition, the document's, and every `<history>`'s default.
/// Every StatePolicy used with StaticExecutionEngine must satisfy this.
///
/// Requires `HierarchyPolicyTrait`; `P::Event` and `P::History`;
/// `P::HAS_PARALLEL_STATES` convertible to `bool`; a valid
/// `P::getDocumentInitialTargets()`; and for a `State s` and a `History h`:
/// `P::initialState()` returning exactly `State`; `P::isFinalState(s)`
/// convertible to `bool`; a valid `P::getInitialTargets(s)`;
/// `P::getHistoryParent(h)` returning exactly `State`; a valid
/// `P::getHistoryDefaultTargets(h)`.
template <typename P, typename = void> struct BaseStatePolicyTrait : std::false_type {};

template <typename P>
struct BaseStatePolicyTrait<P,
                            std::void_t<typename P::Event, typename P::History, detail::HasParallelStatesResult<P>,
                                        detail::GetDocumentInitialTargetsResult<P>, detail::InitialStateResult<P>,
                                        detail::IsFinalStateResult<P>, detail::GetInitialTargetsResult<P>,
                                        detail::GetHistoryParentResult<P>, detail::GetHistoryDefaultTargetsResult<P>>>
    : std::bool_constant<HierarchyPolicyTrait<P>::value &&
                         std::is_convertible_v<detail::HasParallelStatesResult<P>, bool> &&
                         std::is_same_v<detail::InitialStateResult<P>, typename P::State> &&
                         std::is_convertible_v<detail::IsFinalStateResult<P>, bool> &&
                         std::is_same_v<detail::GetHistoryParentResult<P>, typename P::State>> {};

// ─────────────────────────────────────────────────────────────────────────────
// Level 3: Event Naming Policy — event name ↔ enum conversion
// Used by: StaticExecutionEngine (raiseExternal by name, HTTP sends)
// ─────────────────────────────────────────────────────────────────────────────

/// Event name resolution capability.
/// Required for any policy that handles external events or HTTP communication.
///
/// Requires `BaseStatePolicyTrait`, and for a `P p`, an `Event e` and a
/// `const std::string &name`: `p.getEventName(e)` convertible to
/// `std::string`; `p.getEventFromName(name)` returning exactly
/// `std::optional<Event>`.
template <typename P, typename = void> struct EventNamingPolicyTrait : std::false_type {};

template <typename P>
struct EventNamingPolicyTrait<P, std::void_t<detail::GetEventNameResult<P>, detail::GetEventFromNameResult<P>>>
    : std::bool_constant<BaseStatePolicyTrait<P>::value &&
                         std::is_convertible_v<detail::GetEventNameResult<P>, std::string> &&
                         std::is_same_v<detail::GetEventFromNameResult<P>, std::optional<typename P::Event>>> {};

// ─────────────────────────────────────────────────────────────────────────────
// Level 3.5: State Naming Policy — state enum → SCXML id string
// Used by: InPredicateHelper, trace recorders, post-mortem analyzers
// ─────────────────────────────────────────────────────────────────────────────

/// State id string resolution capability.
/// The mapping is structural (independent of parallel states / In() predicate),
/// so every generated policy must provide it. Mirrors EventNamingPolicyTrait's
/// shape — the unconditional `getStateName` emit in utility_methods.jinja2
/// satisfies this contract.
///
/// Requires `BaseStatePolicyTrait`, and for a `State s`: `P::getStateName(s)`
/// convertible to `const char *`.
template <typename P, typename = void> struct StateNamingPolicyTrait : std::false_type {};

template <typename P>
struct StateNamingPolicyTrait<P, std::void_t<detail::GetStateNameResult<P>>>
    : std::bool_constant<BaseStatePolicyTrait<P>::value &&
                         std::is_convertible_v<detail::GetStateNameResult<P>, const char *>> {};

// ─────────────────────────────────────────────────────────────────────────────
// Level 4: Parallel State Policy — parallel state extensions
// Used by: ParallelStateHelper, ParallelCompletionHelper
// ─────────────────────────────────────────────────────────────────────────────

/// Parallel state navigation and ordering capability.
/// Required when HAS_PARALLEL_STATES is true.
///
/// Requires `HierarchyPolicyTrait`, and for `State s1, s2`:
/// `P::isParallelState(s1)`, `P::isDescendantOf(s1, s2)` and
/// `P::isFinalState(s1)` convertible to `bool`; `P::getParallelRegions(s1)`
/// convertible to `std::vector<State>`; `P::getDocumentOrder(s1)` convertible
/// to `int`.
template <typename P, typename = void> struct ParallelStatePolicyTrait : std::false_type {};

template <typename P>
struct ParallelStatePolicyTrait<
    P, std::void_t<detail::IsParallelStateResult<P>, detail::GetParallelRegionsResult<P>,
                   detail::IsDescendantOfResult<P>, detail::GetDocumentOrderResult<P>, detail::IsFinalStateResult<P>>>
    : std::bool_constant<HierarchyPolicyTrait<P>::value &&
                         std::is_convertible_v<detail::IsParallelStateResult<P>, bool> &&
                         std::is_convertible_v<detail::GetParallelRegionsResult<P>, std::vector<typename P::State>> &&
                         std::is_convertible_v<detail::IsDescendantOfResult<P>, bool> &&
                         std::is_convertible_v<detail::GetDocumentOrderResult<P>, int> &&
                         std::is_convertible_v<detail::IsFinalStateResult<P>, bool>> {};

}  // namespace SCE::Core
