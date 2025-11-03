#pragma once

#include <condition_variable>
#include <functional>
#include <memory>
#include <mutex>
#include <queue>

namespace RSM {

// Forward declaration
class EventRaiserImpl;

/**
 * @brief W3C SCXML Platform Abstraction: Event Processing Strategy Helper
 *
 * Zero Duplication Principle: Single Source of Truth for platform-specific
 * event processing logic (WASM synchronous vs Native pthread worker).
 *
 * This Helper abstracts the event processing model differences between platforms:
 * - WASM (Emscripten): Synchronous immediate mode (no pthread support)
 * - Native (Linux/macOS): Pthread-based worker thread for async event processing
 *
 * W3C SCXML 5.3: Asynchronous event processing required for non-blocking state machine operations
 *
 * References:
 * - ARCHITECTURE.md Zero Duplication Principle: Shared logic through Helper functions
 * - ARCHITECTURE.md Helper Function Pattern: SendHelper, ForeachHelper, GuardHelper examples
 * - Similar to PlatformExecutionHelper for JSEngine
 *
 * Usage Example:
 * @code
 * // EventRaiserImpl constructor (platform-agnostic)
 * EventRaiserImpl::EventRaiserImpl(EventCallback callback) {
 *     platformHelper_ = createPlatformEventRaiserHelper(this);
 *     platformHelper_->start();
 * }
 * @endcode
 */
class PlatformEventRaiserHelper {
public:
    virtual ~PlatformEventRaiserHelper() = default;

    /**
     * @brief Start platform-specific event processing infrastructure
     *
     * Platform-specific behavior:
     * - WASM: Enable immediate mode (no worker thread to start)
     * - Native: Start worker thread for async event processing
     *
     * W3C SCXML: Initialize event processing capability
     */
    virtual void start() = 0;

    /**
     * @brief Shutdown platform-specific event processing infrastructure
     *
     * Platform-specific behavior:
     * - WASM: No-op (no worker thread to stop)
     * - Native: Signal worker thread to stop and join
     *
     * W3C SCXML: Clean shutdown of event processing resources
     */
    virtual void shutdown() = 0;

    /**
     * @brief Notify platform-specific infrastructure of new queued event
     *
     * Platform-specific behavior:
     * - WASM: No-op (immediate mode processes synchronously)
     * - Native: Signal condition variable to wake worker thread
     *
     * W3C SCXML: Enable async event processing notification
     */
    virtual void notifyNewEvent() = 0;

    /**
     * @brief Check if event processing should continue
     *
     * Platform-specific behavior:
     * - WASM: Always returns false (no worker thread loop)
     * - Native: Returns true until shutdown requested
     *
     * Used by worker thread main loop condition
     */
    virtual bool shouldProcessEvents() const = 0;

    /**
     * @brief Wait for new events or shutdown signal (for Native worker thread)
     *
     * Platform-specific behavior:
     * - WASM: Not called (no worker thread)
     * - Native: Blocks on condition variable until event or shutdown
     *
     * W3C SCXML: Worker thread blocking for event-driven processing
     */
    virtual void waitForEvents() = 0;
};

/**
 * @brief Factory function to create platform-appropriate event raiser helper
 *
 * Compile-time platform selection:
 * - __EMSCRIPTEN__ defined: Returns SynchronousEventRaiserHelper
 * - __EMSCRIPTEN__ not defined: Returns QueuedEventRaiserHelper
 *
 * @param raiser Pointer to EventRaiserImpl instance (for callbacks)
 * @return std::unique_ptr<PlatformEventRaiserHelper> Platform-specific helper
 *
 * Zero Duplication: Single factory function replaces #ifdef guards throughout EventRaiserImpl
 */
std::unique_ptr<PlatformEventRaiserHelper> createPlatformEventRaiserHelper(EventRaiserImpl *raiser);

}  // namespace RSM
