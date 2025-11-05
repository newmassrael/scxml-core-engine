// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of RSM (Reactive State Machine).
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
// Full terms: https://github.com/newmassrael/reactive-state-machine/blob/main/LICENSE

#pragma once

#include "common/Logger.h"
#include <functional>
#include <vector>

namespace RSM {

/**
 * @brief Helper for onentry/onexit block execution (W3C SCXML 3.8/3.9)
 *
 * Single Source of Truth for block-based entry/exit action execution shared between:
 * - StaticExecutionEngine (AOT engine)
 * - StateMachine (Interpreter engine)
 *
 * ARCHITECTURE.md Compliance:
 * - Zero Duplication Principle: Shared block orchestration logic (lines 311-373)
 * - Single Source of Truth: All block execution centralized in this Helper (lines 320-329)
 * - W3C SCXML 3.8/3.9: Independent onentry/onexit handler execution with error isolation
 *
 * W3C SCXML References:
 * - 3.8: Each <onentry> element is a separate executable content handler
 * - 3.9: Each <onexit> element is a separate executable content handler
 * - 5.10: If error occurs in one handler, remaining handlers MUST still execute
 *
 * Design Pattern:
 * This Helper follows the same pattern as SendHelper, ForeachHelper, GuardHelper:
 * - Template class for policy-based state/event type mapping
 * - Static methods for zero-overhead function calls
 * - Lambda-based action blocks for error isolation
 * - Shared between Interpreter and AOT engines (Zero Duplication)
 *
 * Example Usage (AOT):
 * @code
 * std::vector<std::function<void()>> entryBlocks;
 * entryBlocks.push_back([&]() { // block 1 actions
 * });
 * entryBlocks.push_back([&]() { // block 2 actions
 * });
 *
 * EntryExitHelper<test376Policy, Engine>::executeEntryBlocks(
 *     entryBlocks, engine, "s0"
 * );
 * @endcode
 *
 * Example Usage (Interpreter - Future):
 * @code
 * std::vector<std::function<void()>> blocks;
 * for (const auto& actionBlock : entryBlocks) {
 *     blocks.push_back([&, actionBlock]() {
 *         for (const auto& action : actionBlock) {
 *             if (!action->execute(context)) { return; }
 *         }
 *     });
 * }
 * EntryExitHelper<InterpreterPolicy, InterpreterEngine>::executeEntryBlocks(
 *     blocks, interpreterEngine, stateId
 * );
 * @endcode
 */
template <typename StatePolicy, typename Engine> class EntryExitHelper {
public:
    /**
     * @brief Execute onentry action blocks with error isolation
     *
     * W3C SCXML 3.8: "Each <onentry> element is a separate executable content handler.
     * If an error occurs during execution of an <onentry> handler, the processor MUST
     * cease execution of that handler but MUST continue processing remaining <onentry> handlers."
     *
     * Implementation Details:
     * - Each block is a lambda function wrapping one <onentry> element's actions
     * - If a block raises error.execution and returns, execution stops for THAT block only
     * - Subsequent blocks continue executing (error isolation via lambda scope)
     * - Block execution order: Document order (per W3C SCXML 3.13)
     *
     * @param blocks Vector of block executors (each block = one <onentry> element)
     * @param engine Execution engine for event raising and state management
     * @param stateId State ID for logging (debugging/profiling)
     *
     * @par Thread Safety
     * Thread-safe if engine is accessed exclusively. Blocks executed serially in calling thread.
     *
     * @par Performance
     * O(n) where n = number of blocks. Each block lambda handles its own actions.
     * Zero overhead: Static method with inlining, no virtual calls.
     *
     * @par Error Handling
     * - Block error (e.g., send to invalid target): Stops current block, continues with next
     * - Engine error (e.g., out of memory): Exception propagates to caller
     * - Lambda exception: Not caught, propagates to caller (C++ exception safety)
     *
     * @par ARCHITECTURE.md Compliance
     * - Zero Duplication: Shared logic between Interpreter and AOT engines
     * - Single Source of Truth: Block orchestration centralized in this method
     * - Helper Pattern: Static template method, no state, policy-based types
     *
     * @see executeExitBlocks() for onexit handler execution (W3C SCXML 3.9)
     * @see StateMachine::executeOnEntryActions() for Interpreter implementation (future refactoring target)
     * @see entry_exit_actions.jinja2 for AOT code generation template
     */
    static void executeEntryBlocks(const std::vector<std::function<void()>> &blocks, [[maybe_unused]] Engine &engine,
                                   const std::string &stateId = "") {
        // W3C SCXML 3.8: Log block execution for debugging
        if (!stateId.empty()) {
            LOG_DEBUG("W3C SCXML 3.8: Executing {} onentry blocks for state: {}", blocks.size(), stateId);
        } else {
            LOG_DEBUG("W3C SCXML 3.8: Executing {} onentry blocks", blocks.size());
        }

        // W3C SCXML 3.8: Execute each block independently
        // Block order: Document order (W3C SCXML 3.13)
        for (size_t i = 0; i < blocks.size(); ++i) {
            LOG_DEBUG("W3C SCXML 3.8: Executing onentry block {}/{}", i + 1, blocks.size());

            // W3C SCXML 3.8: Block lambda handles error isolation
            // If block raises error.execution and returns, next block still executes
            // Lambda scope provides natural error boundary (return stops THIS block only)
            blocks[i]();

            // W3C SCXML 3.8: Continue with next block even if previous block had errors
            // This implements "remaining handlers MUST still execute" requirement
        }

        if (!stateId.empty()) {
            LOG_DEBUG("W3C SCXML 3.8: Completed {} onentry blocks for state: {}", blocks.size(), stateId);
        }
    }

    /**
     * @brief Execute onexit action blocks with error isolation
     *
     * W3C SCXML 3.9: "Each <onexit> element is a separate executable content handler."
     * Same semantics as onentry (W3C SCXML 3.8): Independent block execution with error isolation.
     *
     * Implementation Details:
     * - Identical logic to executeEntryBlocks() but for onexit handlers
     * - Each block is a lambda function wrapping one <onexit> element's actions
     * - Error isolation: Block failure stops THAT block only, not subsequent blocks
     * - Block execution order: Document order (per W3C SCXML 3.13)
     *
     * @param blocks Vector of block executors (each block = one <onexit> element)
     * @param engine Execution engine for event raising and state management
     * @param stateId State ID for logging (debugging/profiling)
     *
     * @par Thread Safety
     * Thread-safe if engine is accessed exclusively. Blocks executed serially in calling thread.
     *
     * @par Performance
     * O(n) where n = number of blocks. Each block lambda handles its own actions.
     * Zero overhead: Static method with inlining, no virtual calls.
     *
     * @par Error Handling
     * Same as executeEntryBlocks(): Block errors isolated, engine errors propagate.
     *
     * @par ARCHITECTURE.md Compliance
     * - Zero Duplication: Shared logic between Interpreter and AOT engines
     * - Single Source of Truth: Block orchestration centralized in this method
     * - Helper Pattern: Static template method, no state, policy-based types
     *
     * @see executeEntryBlocks() for onentry handler execution (W3C SCXML 3.8)
     * @see StateExitExecutor::executeExit() for Interpreter implementation (future refactoring target)
     * @see entry_exit_actions.jinja2 for AOT code generation template
     */
    static void executeExitBlocks(const std::vector<std::function<void()>> &blocks, [[maybe_unused]] Engine &engine,
                                  const std::string &stateId = "") {
        // W3C SCXML 3.9: Log block execution for debugging
        if (!stateId.empty()) {
            LOG_DEBUG("W3C SCXML 3.9: Executing {} onexit blocks for state: {}", blocks.size(), stateId);
        } else {
            LOG_DEBUG("W3C SCXML 3.9: Executing {} onexit blocks", blocks.size());
        }

        // W3C SCXML 3.9: Execute each block independently
        // Same logic as onentry blocks (W3C SCXML 3.8)
        for (size_t i = 0; i < blocks.size(); ++i) {
            LOG_DEBUG("W3C SCXML 3.9: Executing onexit block {}/{}", i + 1, blocks.size());

            // W3C SCXML 3.9: Block lambda handles error isolation
            blocks[i]();

            // W3C SCXML 3.9: Continue with next block even if previous block had errors
        }

        if (!stateId.empty()) {
            LOG_DEBUG("W3C SCXML 3.9: Completed {} onexit blocks for state: {}", blocks.size(), stateId);
        }
    }
};

}  // namespace RSM
