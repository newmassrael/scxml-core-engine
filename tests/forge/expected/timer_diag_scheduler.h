// SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_TIMER_DIAG_SCHEDULER_H
#define SCE_FORGE_TIMER_DIAG_SCHEDULER_H

#include <cstdint>
#include <functional>

namespace SCE::Generated::TimerDiagScheduler {

/// Timer callback type.
using TimerCallback = std::function<void()>;

/// Platform timer interface (injected at link time).
struct ITimer {
    virtual ~ITimer() = default;
    virtual void startPeriodic(uint32_t intervalMs, TimerCallback cb) = 0;
    virtual void startOneShot(uint32_t delayMs, TimerCallback cb) = 0;
    virtual void cancel() = 0;
};

struct TimerDiagScheduler {
    ITimer* testerPresentTimer_ = nullptr;
    ITimer* responseTimeoutTimer_ = nullptr;
    ITimer* retryDelayTimer_ = nullptr;

    /// Inject platform timer implementations.
    void init(ITimer* testerPresentTimer, ITimer* responseTimeoutTimer, ITimer* retryDelayTimer) {
        testerPresentTimer_ = testerPresentTimer;
        responseTimeoutTimer_ = responseTimeoutTimer;
        retryDelayTimer_ = retryDelayTimer;
    }

    void startTesterpresent() {
        if (testerPresentTimer_) testerPresentTimer_->startPeriodic(2000, [this]{ onTesterpresent(); });
    }

    void cancelTesterpresent() {
        if (testerPresentTimer_) testerPresentTimer_->cancel();
    }

    void startResponsetimeout() {
        if (responseTimeoutTimer_) responseTimeoutTimer_->startOneShot(5000, [this]{ onHandletimeout(); });
    }

    void cancelResponsetimeout() {
        if (responseTimeoutTimer_) responseTimeoutTimer_->cancel();
    }

    void startRetrydelay() {
        if (retryDelayTimer_) retryDelayTimer_->startOneShot(10000, [this]{ onRetrysecurityaccess(); });
    }

    void cancelRetrydelay() {
        if (retryDelayTimer_) retryDelayTimer_->cancel();
    }

private:
    void onTesterpresent();
    void onHandletimeout();
    void onRetrysecurityaccess();
};

}  // namespace SCE::Generated::TimerDiagScheduler

#endif  // SCE_FORGE_TIMER_DIAG_SCHEDULER_H