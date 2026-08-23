// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $5000 cumulative
//   Enterprise: Contact for pricing
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "quickjs.h"
#include "scripting/ScriptResult.h"
#include <functional>
#include <future>
#include <memory>
#include <optional>

namespace SCE {

/**
 * @brief W3C SCXML Platform Abstraction: Execution Strategy Helper
 *
 * Zero Duplication Principle: Single Source of Truth for platform-specific
 * execution logic (WASM synchronous vs Native pthread queue).
 *
 * This Helper abstracts the execution model differences between platforms:
 * - WASM (Emscripten): Synchronous direct execution (no pthread support)
 * - Native (Linux/macOS): Pthread-based worker queue for thread safety
 *
 * Thread-safe execution required for concurrent state machine instances
 *
 * References:
 * - ARCHITECTURE.md Zero Duplication Principle: Shared logic through Helper functions
 * - ARCHITECTURE.md Helper Function Pattern: SendHelper, ForeachHelper, GuardHelper examples
 *
 * Usage Example:
 * @code
 * // JSEngine method (platform-agnostic)
 * std::future<ScriptResult> JSEngine::executeScript(const std::string& sessionId, const std::string& script) {
 *     return platformExecutor_->executeAsync([this, sessionId, script]() {
 *         return executeScriptInternal(sessionId, script);
 *     });
 * }
 * @endcode
 */
class PlatformExecutionHelper {
public:
    virtual ~PlatformExecutionHelper() = default;

    /**
     * @brief Execute operation asynchronously and return future
     *
     * Platform-specific behavior:
     * - WASM: Execute immediately and wrap result in promise
     * - Native: Queue operation for worker thread execution
     *
     * @param operation Lambda function returning ScriptResult
     * @return std::future<ScriptResult> Future that will contain operation result
     *
     * W3C SCXML: Asynchronous execution for non-blocking state machine operations
     */
    virtual std::future<ScriptResult> executeAsync(std::function<ScriptResult()> operation) = 0;

    /**
     * @brief Run an operation on the engine's thread and answer with ITS type.
     *
     * What this helper decides is WHERE an operation runs — QuickJS is bound
     * to one thread and every session call has to reach it. What an operation
     * answers with is the operation's business, and `executeAsync` above ties
     * the two together because `ScriptResult` was every script call's answer
     * when it was written.
     *
     * `setCurrentEvent` now answers with more than success: it also reports
     * which rung of §scxml-B-2-8-1 the payload got, which is the one fact
     * about a delivered event that nothing can recover afterwards (see
     * `SetCurrentEventResult`). Rather than widen every caller of the virtual,
     * the result travels in a promise this wrapper owns while the virtual goes
     * on deciding the thread.
     *
     * The inner `ScriptResult` is a placeholder the queue's signature requires
     * and nobody reads; the operation's real answer travels beside it.
     *
     * `onRefused` is not a formality. A queued executor whose worker has been
     * joined REFUSES an operation rather than running it — it answers the
     * caller with an error `ScriptResult` and never invokes the function. That
     * refusal is itself the fix for a measured 180-second hang, and a wrapper
     * that assumed its function always runs would replace the hang with a
     * broken promise: `.get()` would throw where every caller expects a value.
     * So the refusing result is handed to `onRefused`, which says what that
     * means for `R`.
     */
    template <typename R>
    std::future<R> executeAsyncReturning(std::function<R()> operation,
                                         std::function<R(const ScriptResult &)> onRefused) {
        auto answered = std::make_shared<std::optional<R>>();
        std::future<ScriptResult> queued = executeAsync([answered, operation]() {
            *answered = operation();
            return ScriptResult::createSuccess();
        });

        // Waiting here does not move when the CALLER waits: `executeAsync`
        // hands back a future, and every caller of every operation on this
        // class `.get()`s it on the next expression. The wait is what lets the
        // refusal above be told apart from a completed run, which is the one
        // thing the queued future knows and the promise does not.
        const ScriptResult queuedResult = queued.get();

        std::promise<R> promise;
        if (answered->has_value()) {
            promise.set_value(std::move(**answered));
        } else {
            promise.set_value(onRefused(queuedResult));
        }
        return promise.get_future();
    }

    /**
     * @brief Shutdown platform-specific execution infrastructure
     *
     * Platform-specific behavior:
     * - WASM: No-op (no worker thread to stop)
     * - Native: Signal worker thread to stop and join
     *
     * W3C SCXML: Clean shutdown of JavaScript engine resources
     */
    virtual void shutdown() = 0;

    /**
     * @brief Reset platform-specific execution infrastructure
     *
     * Platform-specific behavior:
     * - WASM: No-op (no worker thread to restart)
     * - Native: Stop existing worker thread and start new one
     *
     * W3C SCXML: Reset JavaScript engine to initial state
     */
    virtual void reset() = 0;

    /**
     * @brief Get QuickJS runtime pointer created by this executor
     *
     * Platform-specific behavior:
     * - WASM: Runtime created on main thread during construction
     * - Native: Runtime created on worker thread, pointer returned after initialization
     *
     * QuickJS Thread Safety: Runtime must be created and used on same thread
     *
     * @return JSRuntime* Pointer to QuickJS runtime (nullptr if not yet initialized)
     */
    virtual JSRuntime *getRuntimePointer() = 0;

    /**
     * @brief Wait for runtime to be initialized (for Native pthread executor)
     *
     * Platform-specific behavior:
     * - WASM: Returns immediately (runtime created synchronously)
     * - Native: Blocks until worker thread has created runtime
     *
     * W3C SCXML: Ensure runtime is ready before session operations
     */
    virtual void waitForRuntimeInitialization() = 0;
};

/**
 * @brief Factory function to create platform-appropriate executor
 *
 * Compile-time platform selection:
 * - __EMSCRIPTEN__ defined: Returns SynchronousExecutionHelper
 * - __EMSCRIPTEN__ not defined: Returns QueuedExecutionHelper
 *
 * @return std::unique_ptr<PlatformExecutionHelper> Platform-specific executor
 *
 * Zero Duplication: Single factory function replaces #ifdef guards in 19 methods
 */
std::unique_ptr<PlatformExecutionHelper> createPlatformExecutor();

}  // namespace SCE
