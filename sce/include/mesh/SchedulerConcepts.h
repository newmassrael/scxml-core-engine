// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE_MESH.md §mesh-3.1: this file is that section's SCE-side deliverable
// in full. The section splits scheduling into two strategies and says SCE
// publishes the contract while the integrator supplies the scheduler and
// owns the loop; what SCE therefore owes is exactly these predicates, and
// they live nowhere else. Each concept requires the scheduler to name its
// own associated type (`Duration` / `Event`) and to reach its own
// instances, so neither operation takes an instance or event batch —
// taking batches would make SCE the owner of the instance roster, which
// is the thing that differs most between a game loop holding thousands of
// entities and an ECU holding one.
//
// SCE Mesh scheduler concepts — formalizes the two scheduling strategies
// for multi-machine orchestration.
//
// TickScheduling:        Poll-based batch scheduler (game loop, fixed-step).
// EventDrivenScheduling: Reactive scheduler (interrupt-driven, RTOS).
//
// Concept definitions only. SCE ships no scheduler and calls none: the
// application owns its loop and drives the two generated halves
// (`TransportRouter` drain + engine `step()`) from it. These predicates
// exist so an integrator writing generic code over its own scheduler
// types gets a compile error instead of a runtime surprise when a type
// is wired into the wrong loop shape. `tests/mesh/test_mesh_local.cpp`
// pins both directions with `static_assert` against model schedulers.

#pragma once

#if __cpp_concepts >= 202002L
#include <concepts>
#endif

#include <chrono>
#include <type_traits>

namespace SCE::Mesh {

// ═══════════════════════════════════════════════════════════════════════════════
// C++17-compatible feature detection traits
//
// These type traits work in both C++17 and C++20.
// Used in if constexpr for scheduler capability detection.
// ═══════════════════════════════════════════════════════════════════════════════

/// Detect S::tick() method presence.
template <typename S, typename = void> struct HasTickTrait : std::false_type {};

template <typename S> struct HasTickTrait<S, std::void_t<decltype(std::declval<S &>().tick())>> : std::true_type {};

/// Detect S::deadline() -> convertible to S::Duration.
template <typename S, typename = void> struct HasDeadlineTrait : std::false_type {};

template <typename S>
struct HasDeadlineTrait<S, std::void_t<decltype(std::declval<S &>().deadline())>> : std::true_type {};

/// Detect S::onEvent(event) method presence.
template <typename S, typename = void> struct HasOnEventTrait : std::false_type {};

template <typename S>
struct HasOnEventTrait<S, std::void_t<decltype(std::declval<S &>().onEvent(std::declval<typename S::Event>()))>>
    : std::true_type {};

// ═══════════════════════════════════════════════════════════════════════════════
// C++20 concepts + C++17 constexpr bool aliases
//
// When __cpp_concepts is available: full concept definitions for template
// constraints and clear error messages.
// When C++17: constexpr bool aliases from traits for if constexpr usage.
// ═══════════════════════════════════════════════════════════════════════════════

#if __cpp_concepts >= 202002L

// ─────────────────────────────────────────────────────────────────────────────
// TickScheduling — poll-based batch scheduler
//
// Required associated types:
//   Duration  — std::chrono duration type for deadline reporting
//
// Required methods:
//   tick()      — drain pending events across all managed instances
//   deadline()  — next tick deadline (Duration from now)
//
// Used by: cooperative schedulers, game-loop integrations,
//          automotive AUTOSAR Runnable patterns
// ─────────────────────────────────────────────────────────────────────────────

template <typename S>
concept TickScheduling = requires { typename S::Duration; } && requires(S &s) {
    { s.tick() };
    { s.deadline() } -> std::convertible_to<typename S::Duration>;
};

// ─────────────────────────────────────────────────────────────────────────────
// EventDrivenScheduling — reactive event dispatch
//
// Required associated types:
//   Event — event type accepted by the scheduler
//
// Required methods:
//   onEvent(event)  — dispatch a single event to the appropriate instance
//
// Used by: RTOS interrupt handlers, epoll/io_uring event loops,
//          shared-memory transport receive paths
// ─────────────────────────────────────────────────────────────────────────────

template <typename S>
concept EventDrivenScheduling = requires { typename S::Event; } && requires(S &s, typename S::Event event) {
    { s.onEvent(event) };
};

/// Scheduler supports tick-based polling
template <typename S>
concept HasTick = HasTickTrait<S>::value;

/// Scheduler supports deadline reporting
template <typename S>
concept HasDeadline = HasDeadlineTrait<S>::value;

/// Scheduler supports event-driven dispatch
template <typename S>
concept HasOnEvent = HasOnEventTrait<S>::value;

#else  // C++17 fallback

// ─────────────────────────────────────────────────────────────────────────────
// C++17: constexpr bool aliases for if constexpr usage
//
// No concept definitions — template constraints use plain typename.
// Feature detection works identically: if constexpr (HasTick<S>) { ... }
// ─────────────────────────────────────────────────────────────────────────────

template <typename S> inline constexpr bool HasTick = HasTickTrait<S>::value;

template <typename S> inline constexpr bool HasDeadline = HasDeadlineTrait<S>::value;

template <typename S> inline constexpr bool HasOnEvent = HasOnEventTrait<S>::value;

#endif  // __cpp_concepts >= 202002L

}  // namespace SCE::Mesh
