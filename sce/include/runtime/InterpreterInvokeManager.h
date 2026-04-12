// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include "runtime/InvokeExecutor.h"
#include <memory>
#include <string>
#include <vector>

/**
 * @file InterpreterInvokeManager.h
 * @brief Interpreter-specific invoke manager adapter (W3C SCXML 6.4)
 *
 * Separated from core/InvokeManagerAdapters.h to maintain sce_core's
 * header-only boundary. This adapter depends on InvokeExecutor (sce_runtime).
 */

namespace SCE::Core {

// Forward declaration
class StateMachine;

/**
 * @brief Interpreter engine invoke manager adapter
 *
 * Adapts InvokeExecutor (Interpreter's invoke management) to the unified
 * interface required by InvokeProcessingAlgorithms.
 *
 * Implementation notes:
 * - Direct delegation to InvokeExecutor methods
 * - InvokeExecutor handles all complexity (session tracking, finalize scripts)
 * - Adapter is just a thin wrapper for interface unification
 *
 * @example Usage in Interpreter StateMachine.cpp:
 * @code
 * SCE::Core::InterpreterInvokeManager adapter(invokeExecutor_);
 * SCE::Core::InvokeProcessingAlgorithms::processFinalize(
 *     event.originSessionId,
 *     adapter,
 *     *actionExecutor_
 * );
 * @endcode
 */
class InterpreterInvokeManager {
public:
    /**
     * @brief Constructor
     * @param executor InvokeExecutor shared pointer
     */
    explicit InterpreterInvokeManager(std::shared_ptr<InvokeExecutor> executor) : executor_(executor) {}

    /**
     * @brief Get finalize script for child session (W3C SCXML 6.5)
     * @param childSessionId Child session ID that sent event
     * @return Finalize script if exists, empty string otherwise
     */
    std::string getFinalizeScript(const std::string &childSessionId) const {
        if (!executor_) {
            return "";
        }
        return executor_->getFinalizeScriptForChildSession(childSessionId);
    }

    /**
     * @brief Get child sessions with autoforward enabled (W3C SCXML 6.4.1)
     * @param parentSessionId Parent session ID
     * @return Vector of child StateMachine shared_ptrs with autoforward=true
     */
    std::vector<std::shared_ptr<StateMachine>> getAutoforwardSessions(const std::string &parentSessionId) {
        if (!executor_) {
            return {};
        }
        return executor_->getAutoForwardSessions(parentSessionId);
    }

private:
    std::shared_ptr<InvokeExecutor> executor_;
};

}  // namespace SCE::Core
