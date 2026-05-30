// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "events/PlatformEventRaiserHelper.h"
#include "core/LogMacros.h"
#include "events/EventSchedulerImpl.h"
#include "runtime/EventRaiserImpl.h"
#include <atomic>
#include <condition_variable>
#include <memory>
#include <mutex>
#include <thread>

namespace SCE {

// Forward declaration
class IEventScheduler;

/**
 * @brief WASM Synchronous Helper: Immediate mode event processing without threading
 *
 * Zero Duplication: WASM platform logic isolated in this class
 *
 * W3C SCXML: Synchronous immediate processing for single-threaded JavaScript engines
 */
class SynchronousEventRaiserHelper : public PlatformEventRaiserHelper {
private:
    EventRaiserImpl *raiser_ = nullptr;
    std::shared_ptr<IEventScheduler> scheduler_ = nullptr;

public:
    explicit SynchronousEventRaiserHelper(EventRaiserImpl *raiser, std::shared_ptr<IEventScheduler> scheduler)
        : raiser_(raiser), scheduler_(scheduler) {
        SCE_LOG_DEBUG("PlatformEventRaiserHelper: Synchronous helper initialized (WASM mode)");
    }

    ~SynchronousEventRaiserHelper() override {
        SCE_LOG_DEBUG("PlatformEventRaiserHelper: Synchronous helper destroyed");
    }

    void start() override {
        // WASM: Enable immediate mode and set isRunning_ flag
        raiser_->isRunning_.store(true);
        raiser_->setImmediateMode(true);
        SCE_LOG_DEBUG("PlatformEventRaiserHelper: WASM immediate mode enabled, isRunning set to true");
    }

    void shutdown() override {
        // WASM: No worker thread to stop
        SCE_LOG_DEBUG("PlatformEventRaiserHelper: Synchronous helper shutdown (no-op)");
    }

    void notifyNewEvent() override {
        // WASM: No worker thread to notify
    }

    bool shouldProcessEvents() const override {
        // WASM: No worker thread loop
        return false;
    }

    void waitForEvents() override {
        // WASM: Not called (no worker thread)
    }

    void pollScheduler() override {
        // §scxml-6.2: Poll EventScheduler for ready delayed events (WASM synchronous mode)
        // §scxml-3.13: Scheduler always polls automatically (timeout → queue)
        //                 Queue processing is controlled by EventRaiser immediate mode
        if (scheduler_) {
#ifdef __EMSCRIPTEN__
            size_t processedCount = static_cast<EventSchedulerImpl *>(scheduler_.get())->poll();
            if (processedCount > 0) {
                SCE_LOG_DEBUG("PlatformEventRaiserHelper: Scheduler polled, processed {} delayed events", processedCount);
            }
#endif
        }
    }
};

#ifndef __EMSCRIPTEN__

/**
 * @brief Native Pthread Worker Helper: Async event processing with worker thread
 *
 * Zero Duplication: Native platform logic isolated in this class
 *
 * §scxml-5.3: Thread-safe asynchronous event processing for concurrent state machine instances
 *
 * Architecture:
 * - Main thread: Queues events via EventRaiserImpl::raiseEvent()
 * - Worker thread: Processes queued events in order via eventProcessingWorker()
 * - Thread safety: Mutex protects queue, condition variable signals new events
 */
class QueuedEventRaiserHelper : public PlatformEventRaiserHelper {
private:
    EventRaiserImpl *raiser_ = nullptr;
    std::condition_variable *queueCondition_ = nullptr;
    std::mutex *queueMutex_ = nullptr;
    std::atomic<bool> *shutdownRequested_ = nullptr;
    std::atomic<bool> *isRunning_ = nullptr;
    std::thread processingThread_;

public:
    QueuedEventRaiserHelper(EventRaiserImpl *raiser, std::condition_variable *queueCondition, std::mutex *queueMutex,
                            std::atomic<bool> *shutdownRequested, std::atomic<bool> *isRunning)
        : raiser_(raiser), queueCondition_(queueCondition), queueMutex_(queueMutex),
          shutdownRequested_(shutdownRequested), isRunning_(isRunning) {
        SCE_LOG_DEBUG("PlatformEventRaiserHelper: Queued helper initialized (Native pthread mode)");
    }

    ~QueuedEventRaiserHelper() override {
        shutdown();
        SCE_LOG_DEBUG("PlatformEventRaiserHelper: Queued helper destroyed");
    }

    void start() override {
        // Native: Start worker thread for async event processing
        isRunning_->store(true);
        processingThread_ = std::thread(&EventRaiserImpl::eventProcessingWorker, raiser_);
        SCE_LOG_DEBUG("PlatformEventRaiserHelper: Worker thread started");
    }

    void shutdown() override {
        if (!isRunning_->load()) {
            return;  // Already shut down
        }

        SCE_LOG_DEBUG("PlatformEventRaiserHelper: Shutting down worker thread");

        // W3C SCXML: Must hold queueMutex_ when setting shutdownRequested_ to prevent
        // lost notification race with condition_variable::wait() predicate check.
        // Without the mutex, the worker thread can check the predicate (seeing false),
        // then the main thread sets the flag and notifies, then the worker enters wait()
        // and blocks forever because the notification was already sent.
        {
            std::lock_guard<std::mutex> lock(*queueMutex_);
            shutdownRequested_->store(true);
        }
        queueCondition_->notify_all();

        // Wait for worker thread to complete
        if (processingThread_.joinable()) {
            processingThread_.join();
            SCE_LOG_DEBUG("PlatformEventRaiserHelper: Worker thread joined");
        }
    }

    void notifyNewEvent() override {
        // Native: Wake worker thread to process new event
        queueCondition_->notify_one();
    }

    bool shouldProcessEvents() const override {
        // Native: Process events until shutdown requested
        return !shutdownRequested_->load();
    }

    void waitForEvents() override {
        // Native: Block until new event or shutdown signal
        std::unique_lock<std::mutex> lock(*queueMutex_);
        queueCondition_->wait(lock, [this] { return shutdownRequested_->load() || raiser_->hasQueuedEvents(); });
    }

    void pollScheduler() override {
        // Native: No-op - background timer thread handles scheduling automatically
        // §scxml-6.2: EventScheduler timer thread processes delayed events asynchronously
    }
};

#endif  // !__EMSCRIPTEN__

// Factory function implementation
std::unique_ptr<PlatformEventRaiserHelper>
createPlatformEventRaiserHelper(EventRaiserImpl *raiser, [[maybe_unused]] std::shared_ptr<IEventScheduler> scheduler) {
#ifdef __EMSCRIPTEN__
    SCE_LOG_DEBUG("PlatformEventRaiserHelper: Creating synchronous helper (WASM) with scheduler polling");
    return std::make_unique<SynchronousEventRaiserHelper>(raiser, scheduler);
#else
    // Native: scheduler not used (background timer thread handles scheduling)

    SCE_LOG_DEBUG("PlatformEventRaiserHelper: Creating queued helper (Native pthread)");

    // Native helper needs access to EventRaiserImpl's synchronization primitives
    // We'll pass pointers to these members (they're public in EventRaiserImpl.h)
    return std::make_unique<QueuedEventRaiserHelper>(raiser, &raiser->queueCondition_, &raiser->queueMutex_,
                                                     &raiser->shutdownRequested_, &raiser->isRunning_);
#endif
}

}  // namespace SCE
