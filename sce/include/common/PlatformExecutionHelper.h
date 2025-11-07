// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "quickjs.h"
#include "scripting/JSResult.h"
#include <functional>
#include <future>
#include <memory>

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
 * W3C SCXML 5.3: Thread-safe execution required for concurrent state machine instances
 *
 * References:
 * - ARCHITECTURE.md Zero Duplication Principle: Shared logic through Helper functions
 * - ARCHITECTURE.md Helper Function Pattern: SendHelper, ForeachHelper, GuardHelper examples
 *
 * Usage Example:
 * @code
 * // JSEngine method (platform-agnostic)
 * std::future<JSResult> JSEngine::executeScript(const std::string& sessionId, const std::string& script) {
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
     * @param operation Lambda function returning JSResult
     * @return std::future<JSResult> Future that will contain operation result
     *
     * W3C SCXML: Asynchronous execution for non-blocking state machine operations
     */
    virtual std::future<JSResult> executeAsync(std::function<JSResult()> operation) = 0;

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
