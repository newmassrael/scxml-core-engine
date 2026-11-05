// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "events/PlatformEventRaiserHelper.h"
#include "common/LogToLogger.h"
#include "common/ThreadSupport.h"
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

#if !SCE_HAS_THREADS

/**
 * @brief Host-driven helper: immediate-mode event processing on the caller's thread
 *
 * The model of a build without threads (common/ThreadSupport.h): no worker
 * thread exists, so an event is processed when it is raised and a delayed
 * event when the machine's thread polls the scheduler.
 */
class SynchronousEventRaiserHelper : public PlatformEventRaiserHelper {
private:
    EventRaiserImpl *raiser_ = nullptr;
    std::shared_ptr<IEventScheduler> scheduler_ = nullptr;

public:
    explicit SynchronousEventRaiserHelper(EventRaiserImpl *raiser, std::shared_ptr<IEventScheduler> scheduler)
        : raiser_(raiser), scheduler_(scheduler) {
        SCE_LOG_DEBUG("PlatformEventRaiserHelper: Synchronous helper initialized (host-driven)");
    }

    ~SynchronousEventRaiserHelper() override {
        SCE_LOG_DEBUG("PlatformEventRaiserHelper: Synchronous helper destroyed");
    }

    void start() override {
        // No worker thread: enable immediate mode and set the isRunning_ flag
        raiser_->isRunning_.store(true);
        raiser_->setImmediateMode(true);
        SCE_LOG_DEBUG("PlatformEventRaiserHelper: immediate mode enabled, isRunning set to true");
    }

    void shutdown() override {
        // No worker thread to stop
        SCE_LOG_DEBUG("PlatformEventRaiserHelper: Synchronous helper shutdown (no-op)");
    }

    void notifyNewEvent() override {
        // No worker thread to notify
    }

    bool shouldProcessEvents() const override {
        // No worker thread loop
        return false;
    }

    void waitForEvents() override {
        // Not called: no worker thread
    }

    void pollScheduler() override {
        // §scxml-6.2.3: Poll EventScheduler for ready delayed events (host-driven model);
        //               dispatch occurs once the delay interval elapses. Queue processing
        //               is controlled by EventRaiser immediate mode.
        if (scheduler_) {
            size_t processedCount = static_cast<EventSchedulerImpl *>(scheduler_.get())->poll();
            if (processedCount > 0) {
                SCE_LOG_DEBUG("PlatformEventRaiserHelper: Scheduler polled, processed {} delayed events",
                              processedCount);
            }
        }
    }
};

#else  // SCE_HAS_THREADS

/**
 * @brief Threaded helper: async event processing with a worker thread
 *
 * Zero Duplication: the threaded model's logic isolated in this class
 *
 * Thread-safe asynchronous event processing for concurrent state machine instances
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
        SCE_LOG_DEBUG("PlatformEventRaiserHelper: Queued helper initialized (threaded)");
    }

    ~QueuedEventRaiserHelper() override {
        shutdown();
        SCE_LOG_DEBUG("PlatformEventRaiserHelper: Queued helper destroyed");
    }

    void start() override {
        // Start the worker thread for async event processing
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
        // Wake the worker thread to process the new event
        queueCondition_->notify_one();
    }

    bool shouldProcessEvents() const override {
        // Process events until shutdown is requested
        return !shutdownRequested_->load();
    }

    void waitForEvents() override {
        // Block until a new event or the shutdown signal
        std::unique_lock<std::mutex> lock(*queueMutex_);
        queueCondition_->wait(lock, [this] { return shutdownRequested_->load() || raiser_->hasQueuedEvents(); });
    }

    void pollScheduler() override {
        // No-op: the scheduler's timer thread fires delayed events when they fall due
        // §scxml-6.2: EventScheduler timer thread processes delayed events asynchronously
    }
};

#endif  // SCE_HAS_THREADS

// Factory function implementation
std::unique_ptr<PlatformEventRaiserHelper>
createPlatformEventRaiserHelper(EventRaiserImpl *raiser, [[maybe_unused]] std::shared_ptr<IEventScheduler> scheduler) {
#if !SCE_HAS_THREADS
    SCE_LOG_DEBUG("PlatformEventRaiserHelper: Creating synchronous helper (host-driven) with scheduler polling");
    return std::make_unique<SynchronousEventRaiserHelper>(raiser, scheduler);
#else
    // Threaded: the scheduler is not polled (its timer thread fires what falls due)

    SCE_LOG_DEBUG("PlatformEventRaiserHelper: Creating queued helper (threaded)");

    // The threaded helper needs access to EventRaiserImpl's synchronization primitives
    // We'll pass pointers to these members (they're public in EventRaiserImpl.h)
    return std::make_unique<QueuedEventRaiserHelper>(raiser, &raiser->queueCondition_, &raiser->queueMutex_,
                                                     &raiser->shutdownRequested_, &raiser->isRunning_);
#endif
}

}  // namespace SCE
