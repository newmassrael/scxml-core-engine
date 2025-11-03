#include "common/PlatformExecutionHelper.h"
#include "common/Logger.h"
#include "quickjs.h"
#include <condition_variable>
#include <mutex>
#include <queue>
#include <thread>

namespace RSM {

/**
 * @brief WASM Synchronous Executor: Direct function calls without threading
 *
 * Zero Duplication: WASM platform logic isolated in this class
 *
 * W3C SCXML: Synchronous execution model for single-threaded JavaScript engines
 */
class SynchronousExecutionHelper : public PlatformExecutionHelper {
private:
    JSRuntime *runtime_ = nullptr;

public:
    SynchronousExecutionHelper() {
        LOG_DEBUG("PlatformExecutionHelper: Synchronous executor initialized (WASM mode)");

        // W3C SCXML: Create QuickJS runtime on main thread for WASM
        runtime_ = JS_NewRuntime();
        if (!runtime_) {
            LOG_ERROR("PlatformExecutionHelper: Failed to create QuickJS runtime");
        }
    }

    ~SynchronousExecutionHelper() override {
        if (runtime_) {
            JS_FreeRuntime(runtime_);
            runtime_ = nullptr;
        }
        LOG_DEBUG("PlatformExecutionHelper: Synchronous executor destroyed");
    }

    std::future<JSResult> executeAsync(std::function<JSResult()> operation) override {
        // WASM: Execute immediately (no worker thread)
        JSResult result = operation();
        std::promise<JSResult> promise;
        promise.set_value(std::move(result));
        return promise.get_future();
    }

    void shutdown() override {
        // WASM: Free runtime during shutdown (not in destructor)
        // QuickJS: All contexts must be freed before freeing runtime
        if (runtime_) {
            LOG_DEBUG("PlatformExecutionHelper: Synchronous executor - freeing runtime");
            JS_FreeRuntime(runtime_);
            runtime_ = nullptr;
        }
        LOG_DEBUG("PlatformExecutionHelper: Synchronous executor shutdown complete");
    }

    void reset() override {
        // WASM: No worker thread to restart
        LOG_DEBUG("PlatformExecutionHelper: Synchronous executor reset (no-op)");
    }

    JSRuntime *getRuntimePointer() override {
        return runtime_;
    }

    void waitForRuntimeInitialization() override {
        // WASM: Runtime created synchronously, no wait needed
    }
};

#ifndef __EMSCRIPTEN__

/**
 * @brief Native Pthread Queue Executor: Worker thread with request queue
 *
 * Zero Duplication: Native platform logic isolated in this class
 *
 * W3C SCXML 5.3: Thread-safe execution for concurrent state machine instances
 *
 * Architecture:
 * - Main thread: Queues operations via executeAsync()
 * - Worker thread: Processes queued operations in order
 * - Thread safety: Mutex protects queue, condition variable signals new work
 */
class QueuedExecutionHelper : public PlatformExecutionHelper {
private:
    struct QueuedOperation {
        std::function<JSResult()> operation;
        std::promise<JSResult> promise;

        QueuedOperation(std::function<JSResult()> op) : operation(std::move(op)) {}
    };

    JSRuntime *runtime_ = nullptr;
    std::queue<std::unique_ptr<QueuedOperation>> operationQueue_;
    mutable std::mutex queueMutex_;
    mutable std::condition_variable queueCondition_;
    mutable std::condition_variable runtimeInitCondition_;
    std::thread workerThread_;
    std::atomic<bool> shouldStop_{false};
    std::atomic<bool> runtimeInitialized_{false};

    /**
     * @brief Worker thread main loop
     *
     * W3C SCXML: Sequential execution of JavaScript operations
     *
     * QuickJS Thread Safety: Runtime created on worker thread
     */
    void workerLoop() {
        LOG_DEBUG("PlatformExecutionHelper: Worker thread started");

        // W3C SCXML: Create QuickJS runtime on worker thread for thread safety
        runtime_ = JS_NewRuntime();
        if (!runtime_) {
            LOG_ERROR("PlatformExecutionHelper: Failed to create QuickJS runtime on worker thread");
        }

        // Signal that runtime is ready
        {
            std::lock_guard<std::mutex> lock(queueMutex_);
            runtimeInitialized_ = true;
        }
        runtimeInitCondition_.notify_one();
        LOG_DEBUG("PlatformExecutionHelper: Runtime created on worker thread");

        while (true) {
            std::unique_ptr<QueuedOperation> operation;

            // Wait for work or shutdown signal
            {
                std::unique_lock<std::mutex> lock(queueMutex_);
                queueCondition_.wait(lock, [this] { return !operationQueue_.empty() || shouldStop_.load(); });

                if (shouldStop_.load() && operationQueue_.empty()) {
                    LOG_DEBUG("PlatformExecutionHelper: Worker thread stopping");
                    break;
                }

                if (!operationQueue_.empty()) {
                    operation = std::move(operationQueue_.front());
                    operationQueue_.pop();
                }
            }

            // Execute operation outside of lock
            if (operation) {
                try {
                    JSResult result = operation->operation();
                    operation->promise.set_value(std::move(result));
                } catch (const std::exception &e) {
                    LOG_ERROR("PlatformExecutionHelper: Operation failed: {}", e.what());
                    operation->promise.set_value(JSResult::createError(std::string("Exception: ") + e.what()));
                } catch (...) {
                    LOG_ERROR("PlatformExecutionHelper: Operation failed with unknown exception");
                    operation->promise.set_value(JSResult::createError("Unknown exception"));
                }
            }
        }

        // Cleanup runtime on worker thread
        if (runtime_) {
            JS_FreeRuntime(runtime_);
            runtime_ = nullptr;
        }

        LOG_DEBUG("PlatformExecutionHelper: Worker thread stopped");
    }

public:
    QueuedExecutionHelper() : shouldStop_(false), runtimeInitialized_(false) {
        LOG_DEBUG("PlatformExecutionHelper: Queued executor starting worker thread");
        workerThread_ = std::thread(&QueuedExecutionHelper::workerLoop, this);
    }

    ~QueuedExecutionHelper() override {
        shutdown();
        LOG_DEBUG("PlatformExecutionHelper: Queued executor destroyed");
    }

    std::future<JSResult> executeAsync(std::function<JSResult()> operation) override {
        auto queuedOp = std::make_unique<QueuedOperation>(std::move(operation));
        auto future = queuedOp->promise.get_future();

        {
            std::lock_guard<std::mutex> lock(queueMutex_);
            operationQueue_.push(std::move(queuedOp));
        }
        queueCondition_.notify_one();

        return future;
    }

    void shutdown() override {
        LOG_DEBUG("PlatformExecutionHelper: Queued executor shutdown requested");

        if (shouldStop_.load()) {
            LOG_DEBUG("PlatformExecutionHelper: Already shut down");
            return;
        }

        shouldStop_ = true;
        queueCondition_.notify_one();

        if (workerThread_.joinable()) {
            LOG_DEBUG("PlatformExecutionHelper: Joining worker thread");
            workerThread_.join();
            LOG_DEBUG("PlatformExecutionHelper: Worker thread joined");
        }
    }

    void reset() override {
        LOG_DEBUG("PlatformExecutionHelper: Queued executor reset");

        // Stop existing worker
        if (!shouldStop_.load()) {
            shutdown();
        }

        // Clear any remaining operations
        {
            std::lock_guard<std::mutex> lock(queueMutex_);
            while (!operationQueue_.empty()) {
                auto op = std::move(operationQueue_.front());
                operationQueue_.pop();
                // Set error for pending operations
                op->promise.set_value(JSResult::createError("JSEngine reset"));
            }
        }

        // Start new worker (will create new runtime on worker thread)
        shouldStop_ = false;
        runtimeInitialized_ = false;
        workerThread_ = std::thread(&QueuedExecutionHelper::workerLoop, this);
        LOG_DEBUG("PlatformExecutionHelper: Worker thread restarted");
    }

    JSRuntime *getRuntimePointer() override {
        return runtime_;
    }

    void waitForRuntimeInitialization() override {
        // Wait for worker thread to create runtime
        std::unique_lock<std::mutex> lock(queueMutex_);
        runtimeInitCondition_.wait(lock, [this] { return runtimeInitialized_.load(); });
    }
};

#endif  // !__EMSCRIPTEN__

// Factory function implementation
std::unique_ptr<PlatformExecutionHelper> createPlatformExecutor() {
#ifdef __EMSCRIPTEN__
    LOG_DEBUG("PlatformExecutionHelper: Creating synchronous executor (WASM)");
    return std::make_unique<SynchronousExecutionHelper>();
#else
    LOG_DEBUG("PlatformExecutionHelper: Creating queued executor (Native pthread)");
    return std::make_unique<QueuedExecutionHelper>();
#endif
}

}  // namespace RSM
