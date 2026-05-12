// SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
// Shape: watching-zenoh RFC §5.D line 880-886 — single timer per
// doc with event-driven reset / state-exit cancel / fire event.
// Runtime: sce_forge_runtime::hal
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_TIMER_DIAG_SCHEDULER_H
#define SCE_FORGE_TIMER_DIAG_SCHEDULER_H

#include <cstdint>
#include <sce/forge/timer.h>

namespace SCE::Generated::TimerDiagScheduler {

/// Period configured at compile time from `<sce:period>`.
/// Microseconds (uint64_t) — covers MCU microsecond ticks through
/// minute-scale watchdogs in one type.
constexpr std::uint64_t kPeriodUs = 2000000ULL;
constexpr std::uint32_t kPeriodMs = 2000U;
constexpr const char* kResetOnEvent = "diag.heartbeat";
constexpr const char* kCancelOnStateExit = "diag.idle";

/// Handler concept for TimerDiagScheduler.
///
/// The user-supplied `Handler` template parameter must declare:
///     void fireDiagTick();
///
/// Inheriting from a base class is not required — duck typing is
/// sufficient. A missing method produces a precise compile error at
/// template instantiation time, because the trampoline below calls
/// it by name.

template <typename Handler>
class TimerDiagScheduler {
public:
    TimerDiagScheduler(Handler& handler, SCE::Forge::ITimer& timer)
        : handler_(handler), timer_(timer) {}

    /// Start the periodic timer at compile-time `kPeriodMs`.
    void start() {
        timer_.startPeriodic(kPeriodMs, &onDiagTick, this);
    }

    /// Cancel the timer. Idempotent per `SCE::Forge::ITimer` contract.
    void cancel() {
        timer_.cancel();
    }

    /// `<sce:reset-on event="diag.heartbeat"/>` consumer hook —
    /// wire into the host SCXML transition body for this event.
    void onResetDiagHeartbeat() {
        cancel();
        start();
    }

    /// `<sce:cancel-on state-exit="diag.idle"/>`
    /// consumer hook — wire into the host SCXML `<onexit>` for state
    /// `diag.idle`.
    void onCancelDiagIdleExit() {
        cancel();
    }

private:
    Handler& handler_;
    SCE::Forge::ITimer& timer_;

    static void onDiagTick(void* ctx) {
        static_cast<TimerDiagScheduler*>(ctx)->handler_.fireDiagTick();
    }
};

}  // namespace SCE::Generated::TimerDiagScheduler

#endif  // SCE_FORGE_TIMER_DIAG_SCHEDULER_H
