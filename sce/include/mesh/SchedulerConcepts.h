// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE_MESH.md §mesh-3.1: this file is that section's SCE-side deliverable
// in full. The section splits scheduling into two strategies and says SCE
// publishes the contract while the integrator supplies the scheduler and
// owns the loop; what SCE therefore owes is exactly these predicates, and
// they live nowhere else. Each predicate requires the scheduler to name its
// own associated type (`Duration` / `Event`) and to reach its own
// instances, so neither operation takes an instance or event batch —
// taking batches would make SCE the owner of the instance roster, which
// is the thing that differs most between a game loop holding thousands of
// entities and an ECU holding one.
//
// SCE Mesh scheduler contract — formalizes the two scheduling strategies
// for multi-machine orchestration.
//
// TickSchedulingTrait:        Poll-based batch scheduler (game loop, fixed-step).
// EventDrivenSchedulingTrait: Reactive scheduler (interrupt-driven, RTOS).
//
// Trait definitions only. SCE ships no scheduler and calls none: the
// application owns its loop and drives the two generated halves
// (`TransportRouter` drain + engine `step()`) from it. These predicates
// exist so an integrator writing generic code over its own scheduler
// types gets a compile error instead of a runtime surprise when a type
// is wired into the wrong loop shape: `static_assert` the trait's `::value`
// as the first line of the template that takes the scheduler.
// `tests/mesh/test_mesh_local.cpp` pins both directions with `static_assert`
// against model schedulers.
//
// Each is a C++17 trait, which means the same thing in a consumer's C++17
// translation unit and in SCE's C++20 ones (docs/adr/0059).

#pragma once

#include <chrono>
#include <type_traits>
#include <utility>

namespace SCE::Mesh {

// ═══════════════════════════════════════════════════════════════════════════════
// Capability detection traits
//
// Asked through `::value` in `if constexpr` for scheduler capability detection.
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

// ─────────────────────────────────────────────────────────────────────────────
// TickSchedulingTrait — poll-based batch scheduler
//
// Required associated types:
//   Duration  — std::chrono duration type for deadline reporting
//
// Required methods, on an `S &s`:
//   s.tick()      — drain pending events across all managed instances
//   s.deadline()  — next tick deadline, convertible to S::Duration (from now)
//
// Used by: cooperative schedulers, game-loop integrations,
//          automotive AUTOSAR Runnable patterns
// ─────────────────────────────────────────────────────────────────────────────

template <typename S, typename = void> struct TickSchedulingTrait : std::false_type {};

template <typename S>
struct TickSchedulingTrait<S, std::void_t<typename S::Duration, decltype(std::declval<S &>().tick()),
                                          decltype(std::declval<S &>().deadline())>>
    : std::is_convertible<decltype(std::declval<S &>().deadline()), typename S::Duration> {};

// ─────────────────────────────────────────────────────────────────────────────
// EventDrivenSchedulingTrait — reactive event dispatch
//
// Required associated types:
//   Event — event type accepted by the scheduler
//
// Required methods, on an `S &s` and an `S::Event event` lvalue:
//   s.onEvent(event)  — dispatch a single event to the appropriate instance
//
// Used by: RTOS interrupt handlers, epoll/io_uring event loops,
//          shared-memory transport receive paths
// ─────────────────────────────────────────────────────────────────────────────

template <typename S, typename = void> struct EventDrivenSchedulingTrait : std::false_type {};

template <typename S>
struct EventDrivenSchedulingTrait<
    S, std::void_t<typename S::Event, decltype(std::declval<S &>().onEvent(std::declval<typename S::Event &>()))>>
    : std::true_type {};

}  // namespace SCE::Mesh
