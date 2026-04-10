// SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_TIMER_DIAG_SCHEDULER_H
#define SCE_FORGE_TIMER_DIAG_SCHEDULER_H

#include <cstdint>
#include <sce/forge/timer.h>

namespace SCE::Generated::TimerDiagScheduler {

class TimerDiagScheduler {
public:
    TimerDiagScheduler(sce::forge::ITimer& testerPresentTimer, sce::forge::ITimer& responseTimeoutTimer, sce::forge::ITimer& retryDelayTimer)
        : testerPresentTimer_(testerPresentTimer), responseTimeoutTimer_(responseTimeoutTimer), retryDelayTimer_(retryDelayTimer) {}

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
    sce::forge::ITimer& testerPresentTimer_;
    sce::forge::ITimer& responseTimeoutTimer_;
    sce::forge::ITimer& retryDelayTimer_;

    static void onTesterPresent(void* ctx) {
        static_cast<TimerDiagScheduler*>(ctx)->fireTesterPresent();
    }
    static void onHandleTimeout(void* ctx) {
        static_cast<TimerDiagScheduler*>(ctx)->fireHandleTimeout();
    }
    static void onRetrySecurityAccess(void* ctx) {
        static_cast<TimerDiagScheduler*>(ctx)->fireRetrySecurityAccess();
    }

    void fireTesterPresent();
    void fireHandleTimeout();
    void fireRetrySecurityAccess();
};

}  // namespace SCE::Generated::TimerDiagScheduler

#endif  // SCE_FORGE_TIMER_DIAG_SCHEDULER_H