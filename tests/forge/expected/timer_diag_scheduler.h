// SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
// Runtime: sce_forge_runtime::hal
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_TIMER_DIAG_SCHEDULER_H
#define SCE_FORGE_TIMER_DIAG_SCHEDULER_H

#include <cstdint>
#include <sce/forge/timer.h>

namespace SCE::Generated::TimerDiagScheduler {

/// Handler concept for TimerDiagScheduler.
///
/// The user-supplied `Handler` template parameter must declare these member
/// functions. Missing methods produce a precise compile error at template
/// instantiation time, because the trampolines below call them by name.
///
/// Required interface:
///     void fireTesterPresent();   // TesterPresent
///     void fireHandleTimeout();   // handleTimeout
///     void fireRetrySecurityAccess();   // retrySecurityAccess
///
/// Inheriting from a base class is not required — duck typing is sufficient.

template <typename Handler>
class TimerDiagScheduler {
public:
    TimerDiagScheduler(Handler& handler, SCE::Forge::ITimer& testerPresentTimer, SCE::Forge::ITimer& responseTimeoutTimer, SCE::Forge::ITimer& retryDelayTimer)
        : handler_(handler), testerPresentTimer_(testerPresentTimer), responseTimeoutTimer_(responseTimeoutTimer), retryDelayTimer_(retryDelayTimer) {}

    void startTesterPresent() {
        testerPresentTimer_.startPeriodic(2000, &onTesterPresent, this);
    }

    void cancelTesterPresent() {
        testerPresentTimer_.cancel();
    }

    void startResponseTimeout() {
        responseTimeoutTimer_.startOneShot(5000, &onHandleTimeout, this);
    }

    void cancelResponseTimeout() {
        responseTimeoutTimer_.cancel();
    }

    void startRetryDelay() {
        retryDelayTimer_.startOneShot(10000, &onRetrySecurityAccess, this);
    }

    void cancelRetryDelay() {
        retryDelayTimer_.cancel();
    }

private:
    Handler& handler_;
    SCE::Forge::ITimer& testerPresentTimer_;
    SCE::Forge::ITimer& responseTimeoutTimer_;
    SCE::Forge::ITimer& retryDelayTimer_;

    static void onTesterPresent(void* ctx) {
        static_cast<TimerDiagScheduler*>(ctx)->handler_.fireTesterPresent();
    }
    static void onHandleTimeout(void* ctx) {
        static_cast<TimerDiagScheduler*>(ctx)->handler_.fireHandleTimeout();
    }
    static void onRetrySecurityAccess(void* ctx) {
        static_cast<TimerDiagScheduler*>(ctx)->handler_.fireRetrySecurityAccess();
    }
};

}  // namespace SCE::Generated::TimerDiagScheduler

#endif  // SCE_FORGE_TIMER_DIAG_SCHEDULER_H
