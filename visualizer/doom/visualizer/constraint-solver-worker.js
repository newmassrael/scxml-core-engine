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

// Import the constraint solver and optimizer modules (dependency order)
importScripts(
    'logger.js',
    'routing-state.js',
    'optimizer/snap-calculator.js',
    'optimizer/path-utils.js',
    'optimizer/csp-solver.js',
    'optimizer/optimizer-core.js',
    'constraint-solver.js'
);

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
    const { type, links, nodes, optimizerState, greedySolution, parentChildMap, draggedNodeId } = e.data;

    switch (type) {
        case 'solve':
            try {
                logger.debug('[WORKER] Starting progressive CSP refinement (8 iterations × 250ms)...');

                // Create optimizer instance
                const optimizer = new TransitionLayoutOptimizer(nodes, links);

                // Reconstruct parent-child map
                const parentChildMapObj = new Map();
                if (parentChildMap) {
                    for (const [child, parent] of Object.entries(parentChildMap)) {
                        parentChildMapObj.set(child, parent);
                    }
                }

                // Reconstruct greedy solution if provided
                let currentBestSolution = null;
                if (greedySolution) {
                    currentBestSolution = {
                        assignment: new Map(Object.entries(greedySolution.assignment || {})),
                        score: greedySolution.score,
                        preferences: new Map(Object.entries(greedySolution.preferences || {}))
                    };
                    logger.debug(`[WORKER] Initial solution: score=${currentBestSolution.score.toFixed(1)}, ${currentBestSolution.preferences.size} preferences`);
                }

                // Progressive refinement: 8 iterations × 250ms = 2000ms total
                const MAX_ITERATIONS = 8;
                let bestScoreOverall = currentBestSolution ? currentBestSolution.score : Infinity;
                let bestSolutionOverall = null;

                for (let iteration = 0; iteration < MAX_ITERATIONS; iteration++) {
                    try {
                        logger.debug(`[WORKER] === Iteration ${iteration + 1}/${MAX_ITERATIONS} ===`);

                        // Create CSP solver with current best solution as warm-start
                        currentSolver = new ConstraintSolver(links, nodes, optimizer, parentChildMapObj, currentBestSolution, draggedNodeId);

                        // Register progress callback with batching
                        currentSolver.onProgressCallback = (progressData) => {
                            const now = performance.now();

                            // Critical update: solution improved
                            // Send immediately for instant visual feedback
                            if (progressData.type === 'solution_improved') {
                                logger.debug(`[WORKER] Iteration ${iteration + 1}: Solution improved: score=${progressData.score.toFixed(1)}`);

                                // Flush any buffered updates first
                                flushProgressBuffer();

                                // Send critical update immediately with iteration info
                                self.postMessage({
                                    type: 'progress',
                                    data: progressData,
                                    iteration: iteration + 1,
                                    totalIterations: MAX_ITERATIONS
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

                        // Solve for 250ms
                        const solution = currentSolver.solve();

                        // Flush any remaining buffered progress
                        flushProgressBuffer();

                        // Check if cancelled
                        if (currentSolver.cancelled) {
                            logger.debug('[WORKER] Solver was cancelled during iteration', iteration + 1);
                            self.postMessage({ type: 'cancelled' });
                            currentSolver = null;
                            return;
                        }

                        // Update best solution if improved
                        if (solution && currentSolver.bestScore < bestScoreOverall) {
                            bestScoreOverall = currentSolver.bestScore;
                            bestSolutionOverall = solution;

                            logger.debug(`[WORKER] Iteration ${iteration + 1}: New best score=${bestScoreOverall.toFixed(1)}`);

                            // Convert solution to warm-start format for next iteration
                            currentBestSolution = {
                                assignment: solution,
                                score: currentSolver.bestScore,
                                preferences: new Map()
                            };

                            // Extract preferences from solution
                            solution.forEach((assignment, linkId) => {
                                currentBestSolution.preferences.set(linkId, {
                                    sourceEdge: assignment.sourceEdge,
                                    targetEdge: assignment.targetEdge
                                });
                            });
                        } else {
                            logger.debug(`[WORKER] Iteration ${iteration + 1}: No improvement (current best: ${bestScoreOverall.toFixed(1)})`);
                        }

                        // Send intermediate progress update EVERY iteration (even if no improvement)
                        if (bestSolutionOverall) {
                            const solutionArray = Array.from(bestSolutionOverall.entries()).map(([linkId, assignment]) => ({
                                linkId,
                                sourceEdge: assignment.sourceEdge,
                                targetEdge: assignment.targetEdge
                            }));

                            self.postMessage({
                                type: 'intermediate_solution',
                                solution: solutionArray,
                                score: bestScoreOverall,
                                iteration: iteration + 1,
                                totalIterations: MAX_ITERATIONS,
                                improved: solution && currentSolver.bestScore === bestScoreOverall,
                                stats: {
                                    nodeCount: currentSolver.nodeCount,
                                    pruneCount: currentSolver.pruneCount
                                }
                            });
                        }

                        currentSolver = null;

                    } catch (iterationError) {
                        console.error(`[WORKER] Error in iteration ${iteration + 1}:`, iterationError);

                        // Send error message to main thread
                        self.postMessage({
                            type: 'error',
                            message: `Iteration ${iteration + 1} failed: ${iterationError.message}`,
                            stack: iterationError.stack,
                            iteration: iteration + 1
                        });

                        // Clean up solver
                        currentSolver = null;

                        // Continue with next iteration if we have a best solution
                        if (bestSolutionOverall) {
                            logger.debug(`[WORKER] Continuing with best solution from previous iterations`);
                            continue;
                        } else {
                            // No best solution yet, abort
                            console.error('[WORKER] No best solution available, aborting');
                            self.postMessage({ type: 'failed' });
                            return;
                        }
                    }
                }

                // Send final solution
                if (bestSolutionOverall) {
                    const solutionArray = Array.from(bestSolutionOverall.entries()).map(([linkId, assignment]) => ({
                        linkId,
                        sourceEdge: assignment.sourceEdge,
                        targetEdge: assignment.targetEdge
                    }));

                    logger.debug(`[WORKER] All iterations complete. Final solution: score=${bestScoreOverall.toFixed(1)}, ${solutionArray.length} links`);

                    self.postMessage({
                        type: 'solution',
                        solution: solutionArray,
                        score: bestScoreOverall,
                        stats: {
                            totalIterations: MAX_ITERATIONS
                        }
                    });
                } else {
                    logger.debug('[WORKER] No solution found after all iterations');
                    self.postMessage({ type: 'failed' });
                }

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
            logger.debug('[WORKER] Cancel requested');
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
