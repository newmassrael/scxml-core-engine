// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Web Worker for CSP Solver (Enhanced with Progress Reporting)
 * Runs ConstraintSolver in a separate thread to avoid blocking main thread
 *
 * Features:
 * - Progressive feedback on solution improvements
 * - Batched progress updates to reduce overhead
 * - Immediate critical updates (solution_improved)
 */

// Import the constraint solver (will be handled by Worker context)
importScripts('constraint-solver.js', 'transition-layout-optimizer.js');

let currentSolver = null;

// Batching variables
let progressBuffer = [];
let lastFlushTime = 0;
const FLUSH_INTERVAL = 100; // 100ms - balance between responsiveness and overhead

/**
 * Flush buffered progress updates to main thread
 * Sends only the latest update to avoid redundant messages
 */
function flushProgressBuffer() {
    if (progressBuffer.length === 0) return;

    // Send only the latest update (most recent state)
    const latest = progressBuffer[progressBuffer.length - 1];

    self.postMessage({
        type: 'progress',
        data: latest
    });

    // Clear buffer
    progressBuffer = [];
    lastFlushTime = performance.now();
}

/**
 * Message handler
 * Expected message format:
 * {
 *   type: 'solve',
 *   links: [...],
 *   nodes: [...],
 *   optimizerState: { ... }  // Serialized optimizer state (optional)
 * }
 */
self.onmessage = function(e) {
    const { type, links, nodes, optimizerState } = e.data;

    switch (type) {
        case 'solve':
            try {
                console.log('[WORKER] Creating optimizer and solver...');

                // Create optimizer instance
                const optimizer = new TransitionLayoutOptimizer(nodes, links);

                // Create CSP solver
                currentSolver = new ConstraintSolver(links, nodes, optimizer);

                // Register progress callback with batching
                currentSolver.onProgressCallback = (progressData) => {
                    const now = performance.now();

                    // Critical update: solution improved
                    // Send immediately for instant visual feedback
                    if (progressData.type === 'solution_improved') {
                        console.log(`[WORKER] Solution improved: score=${progressData.score.toFixed(1)}`);

                        // Flush any buffered updates first
                        flushProgressBuffer();

                        // Send critical update immediately
                        self.postMessage({
                            type: 'progress',
                            data: progressData
                        });
                        return;
                    }

                    // Regular updates: buffer and flush periodically
                    progressBuffer.push(progressData);

                    // Flush if interval exceeded
                    if (now - lastFlushTime > FLUSH_INTERVAL) {
                        flushProgressBuffer();
                    }
                };

                console.log('[WORKER] Starting solve...');

                // Solve
                const solution = currentSolver.solve();

                console.log('[WORKER] Solve completed');

                // Flush any remaining buffered progress
                flushProgressBuffer();

                if (currentSolver.cancelled) {
                    console.log('[WORKER] Solver was cancelled');
                    self.postMessage({ type: 'cancelled' });
                } else if (solution) {
                    // Convert Map to Array for serialization
                    const solutionArray = Array.from(solution.entries()).map(([linkId, assignment]) => ({
                        linkId,
                        sourceEdge: assignment.sourceEdge,
                        targetEdge: assignment.targetEdge
                    }));

                    console.log(`[WORKER] Sending final solution (${solutionArray.length} links)`);

                    self.postMessage({
                        type: 'solution',
                        solution: solutionArray,
                        score: currentSolver.bestScore,
                        stats: {
                            nodeCount: currentSolver.nodeCount,
                            pruneCount: currentSolver.pruneCount
                        }
                    });
                } else {
                    console.log('[WORKER] No solution found');
                    self.postMessage({ type: 'failed' });
                }

                currentSolver = null;
            } catch (error) {
                console.error('[WORKER] Error:', error);
                self.postMessage({
                    type: 'error',
                    message: error.message,
                    stack: error.stack
                });
                currentSolver = null;
            }
            break;

        case 'cancel':
            console.log('[WORKER] Cancel requested');
            if (currentSolver) {
                currentSolver.cancel();
                currentSolver = null;
            }
            // Flush any buffered progress before cancelling
            flushProgressBuffer();
            self.postMessage({ type: 'cancelled' });
            break;

        default:
            self.postMessage({
                type: 'error',
                message: `Unknown message type: ${type}`
            });
    }
};
