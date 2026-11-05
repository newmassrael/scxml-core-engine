// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <condition_variable>
#include <functional>
#include <memory>
#include <mutex>
#include <queue>

namespace SCE {

// Forward declaration
class EventRaiserImpl;

/**
 * @brief W3C SCXML Platform Abstraction: Event Processing Strategy Helper
 *
 * Zero Duplication Principle: Single Source of Truth for the event processing
 * logic of the two execution models (common/ThreadSupport.h).
 *
 * This Helper abstracts the difference between them:
 * - Host-driven (a build without threads): synchronous immediate mode
 * - Threaded (native, or Emscripten with pthreads): a worker thread for async event processing
 *
 * Asynchronous event processing required for non-blocking state machine operations
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
     * - Host-driven: Enable immediate mode (no worker thread to start)
     * - Threaded: Start worker thread for async event processing
     *
     * W3C SCXML: Initialize event processing capability
     */
    virtual void start() = 0;

    /**
     * @brief Shutdown platform-specific event processing infrastructure
     *
     * Platform-specific behavior:
     * - Host-driven: No-op (no worker thread to stop)
     * - Threaded: Signal worker thread to stop and join
     *
     * W3C SCXML: Clean shutdown of event processing resources
     */
    virtual void shutdown() = 0;

    /**
     * @brief Notify platform-specific infrastructure of new queued event
     *
     * Platform-specific behavior:
     * - Host-driven: No-op (immediate mode processes synchronously)
     * - Threaded: Signal condition variable to wake worker thread
     *
     * W3C SCXML: Enable async event processing notification
     */
    virtual void notifyNewEvent() = 0;

    /**
     * @brief Check if event processing should continue
     *
     * Platform-specific behavior:
     * - Host-driven: Always returns false (no worker thread loop)
     * - Threaded: Returns true until shutdown requested
     *
     * Used by worker thread main loop condition
     */
    virtual bool shouldProcessEvents() const = 0;

    /**
     * @brief Wait for new events or shutdown signal (for the threaded model's worker)
     *
     * Platform-specific behavior:
     * - Host-driven: Not called (no worker thread)
     * - Threaded: Blocks on condition variable until event or shutdown
     *
     * W3C SCXML: Worker thread blocking for event-driven processing
     */
    virtual void waitForEvents() = 0;

    /**
     * @brief Poll EventScheduler for ready delayed events
     *
     * Platform-specific behavior:
     * - Host-driven: Calls scheduler->poll() to process delayed events synchronously
     * - Threaded: No-op (background timer thread handles scheduling automatically)
     *
     * §scxml-6.2: Delayed send element support across platforms
     *
     * Zero Duplication: Single Source of Truth for scheduler polling logic
     */
    virtual void pollScheduler() = 0;
};

// Forward declaration
class IEventScheduler;

/**
 * @brief Factory function to create platform-appropriate event raiser helper
 *
 * Compile-time selection by SCE_HAS_THREADS (common/ThreadSupport.h):
 * - 0: Returns SynchronousEventRaiserHelper (with scheduler polling)
 * - 1: Returns QueuedEventRaiserHelper (scheduler handled by its timer thread)
 *
 * @param raiser Pointer to EventRaiserImpl instance (for callbacks)
 * @param scheduler Shared pointer to EventScheduler (polled for delayed events when there are no threads)
 * @return std::unique_ptr<PlatformEventRaiserHelper> Platform-specific helper
 *
 * Zero Duplication: Single factory function replaces #ifdef guards throughout EventRaiserImpl
 */
std::unique_ptr<PlatformEventRaiserHelper> createPlatformEventRaiserHelper(EventRaiserImpl *raiser,
                                                                           std::shared_ptr<IEventScheduler> scheduler);

}  // namespace SCE
