#include "events/PlatformEventRaiserHelper.h"
#include "common/Logger.h"
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
        LOG_DEBUG("PlatformEventRaiserHelper: Synchronous helper initialized (WASM mode)");
    }

    ~SynchronousEventRaiserHelper() override {
        LOG_DEBUG("PlatformEventRaiserHelper: Synchronous helper destroyed");
    }

    void start() override {
        // WASM: Enable immediate mode and set isRunning_ flag
        raiser_->isRunning_.store(true);
        raiser_->setImmediateMode(true);
        LOG_DEBUG("PlatformEventRaiserHelper: WASM immediate mode enabled, isRunning set to true");
    }

    void shutdown() override {
        // WASM: No worker thread to stop
        LOG_DEBUG("PlatformEventRaiserHelper: Synchronous helper shutdown (no-op)");
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
        // W3C SCXML 6.2: Poll EventScheduler for ready delayed events (WASM synchronous mode)
        if (scheduler_) {
#ifdef __EMSCRIPTEN__
            size_t processedCount = static_cast<EventSchedulerImpl *>(scheduler_.get())->poll();
            if (processedCount > 0) {
                LOG_DEBUG("PlatformEventRaiserHelper: Scheduler polled, processed {} delayed events", processedCount);
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
 * W3C SCXML 5.3: Thread-safe asynchronous event processing for concurrent state machine instances
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
        LOG_DEBUG("PlatformEventRaiserHelper: Queued helper initialized (Native pthread mode)");
    }

    ~QueuedEventRaiserHelper() override {
        shutdown();
        LOG_DEBUG("PlatformEventRaiserHelper: Queued helper destroyed");
    }

    void start() override {
        // Native: Start worker thread for async event processing
        isRunning_->store(true);
        processingThread_ = std::thread(&EventRaiserImpl::eventProcessingWorker, raiser_);
        LOG_DEBUG("PlatformEventRaiserHelper: Worker thread started");
    }

    void shutdown() override {
        if (!isRunning_->load()) {
            return;  // Already shut down
        }

        LOG_DEBUG("PlatformEventRaiserHelper: Shutting down worker thread");

        // Signal shutdown
        shutdownRequested_->store(true);
        queueCondition_->notify_all();

        // Wait for worker thread to complete
        if (processingThread_.joinable()) {
            processingThread_.join();
            LOG_DEBUG("PlatformEventRaiserHelper: Worker thread joined");
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
        // W3C SCXML 6.2: EventScheduler timer thread processes delayed events asynchronously
    }
};

#endif  // !__EMSCRIPTEN__

// Factory function implementation
std::unique_ptr<PlatformEventRaiserHelper> createPlatformEventRaiserHelper(EventRaiserImpl *raiser,
                                                                           std::shared_ptr<IEventScheduler> scheduler) {
#ifdef __EMSCRIPTEN__
    LOG_DEBUG("PlatformEventRaiserHelper: Creating synchronous helper (WASM) with scheduler polling");
    return std::make_unique<SynchronousEventRaiserHelper>(raiser, scheduler);
#else
    // Native: scheduler not used (background timer thread handles scheduling)
    (void)scheduler;

    LOG_DEBUG("PlatformEventRaiserHelper: Creating queued helper (Native pthread)");

    // Native helper needs access to EventRaiserImpl's synchronization primitives
    // We'll pass pointers to these members (they're public in EventRaiserImpl.h)
    return std::make_unique<QueuedEventRaiserHelper>(raiser, &raiser->queueCondition_, &raiser->queueMutex_,
                                                     &raiser->shutdownRequested_, &raiser->isRunning_);
#endif
}

}  // namespace SCE
