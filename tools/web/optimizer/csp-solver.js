// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * CSP Solver - Handles constraint satisfaction optimization and worker management
 */

class CSPSolver {
    constructor(optimizer) {
        this.optimizer = optimizer;
    }

    /**
     * Get visual source ID (for collapsed state redirect)
     * @param {Object} link - Link object with optional visualSource
     * @returns {string} Visual source node ID
     */
    getVisualSource(link) {
        return link.visualSource || link.source;
    }

    /**
     * Get visual target ID (for collapsed state redirect)
     * @param {Object} link - Link object with optional visualTarget
     * @returns {string} Visual target node ID
     */
    getVisualTarget(link) {
        return link.visualTarget || link.target;
    }

    _handleProgressMessage(data, links, nodes, lastProgressRender) {
        if (data.data && data.data.type === 'solution_improved') {
            const now = performance.now();
            if (now - lastProgressRender >= TransitionLayoutOptimizer.PROGRESS_RENDER_INTERVAL_MS) {
                console.log(`[OPTIMIZE PROGRESSIVE] Solution improved: score=${data.data.score.toFixed(1)}, progress=${(data.data.progress * 100).toFixed(1)}%`);
                this.optimizer._applySolutionToLinks(data.data.assignment, links, nodes);
                return now;
            }
        }
        return lastProgressRender;
    }

    _handleSolutionMessage(solution, score, stats, links, nodes, worker, onComplete) {
        console.log(`[OPTIMIZE PROGRESSIVE] CSP solution received (score=${score}, nodes=${stats.nodeCount}, prunes=${stats.pruneCount})`);
        console.log('[OPTIMIZE PROGRESSIVE] Applying CSP solution...');
        this.optimizer._applySolutionToLinks(solution, links, nodes);
        console.log('[OPTIMIZE PROGRESSIVE] Background CSP complete!');
        worker.terminate();
        if (onComplete) onComplete(true);
        return null;
    }

    _handleCancelledMessage(worker, onComplete) {
        console.log('[OPTIMIZE PROGRESSIVE] CSP cancelled by user');
        worker.terminate();
        if (onComplete) onComplete(false);
        return null;
    }

    _handleFailedMessage(worker, onComplete) {
        console.log('[OPTIMIZE PROGRESSIVE] CSP failed, keeping greedy result');
        worker.terminate();
        if (onComplete) onComplete(false);
        return null;
    }

    _handleErrorMessage(message, stack, worker, onComplete) {
        console.error('[OPTIMIZE PROGRESSIVE] Worker error:', message);
        console.error(stack);
        worker.terminate();
        if (onComplete) onComplete(false);
        return null;
    }

    optimizeSnapPointAssignmentsProgressive(links, nodes, draggedNodeId, onComplete, onProgress = null, debounceMs = 500) {
        // Filter out containment and delegation links (only optimize transition and initial links)
        // Visualizer layout: containment is hierarchical structure, not routing path
        const transitionLinks = links.filter(link => 
            link.linkType === 'transition' || link.linkType === 'initial'
        );

        console.log('[OPTIMIZE PROGRESSIVE] Starting progressive optimization...');

        // Progressive optimization: greedy step for immediate feedback
        console.log('[OPTIMIZE PROGRESSIVE] Greedy step: immediate rendering...');
        this.optimizer.optimizeSnapPointAssignmentsGreedy(transitionLinks, nodes, draggedNodeId);

        // Return immediately - user sees greedy result
        console.log('[OPTIMIZE PROGRESSIVE] Greedy step complete, scheduling background CSP refinement...');

        // Progressive optimization: CSP refinement with Web Worker
        let timeoutId = null;
        let worker = null;

        const cancel = () => {
            if (timeoutId) {
                clearTimeout(timeoutId);
                timeoutId = null;
                console.log('[OPTIMIZE PROGRESSIVE] Cancelled (timeout cleared)');
            }
            if (worker) {
                worker.postMessage({ type: 'cancel' });
                worker.terminate();
                worker = null;
                console.log('[OPTIMIZE PROGRESSIVE] Cancelled (Worker terminated)');
            }
        };

        // Skip CSP for large state machines (threshold)
        if (transitionLinks.length > TransitionLayoutOptimizer.CSP_THRESHOLD) {
            console.log(`[OPTIMIZE PROGRESSIVE] ${transitionLinks.length} transitions exceed threshold, skipping background CSP`);
            if (onComplete) onComplete(false);
            return { cancel };
        }

        // Schedule background CSP with Web Worker
        timeoutId = setTimeout(() => {
            timeoutId = null;
            console.log('[OPTIMIZE PROGRESSIVE] CSP refinement step: Starting background optimization with Web Worker...');

            // Check if Worker is supported
            if (typeof Worker === 'undefined') {
                console.warn('[OPTIMIZE PROGRESSIVE] Web Workers not supported, falling back to main thread CSP');
                this.fallbackToMainThreadCSP(transitionLinks, nodes, draggedNodeId, onComplete);
                return;
            }

            try {
                // Create Web Worker
                worker = new Worker('constraint-solver-worker.js');

                // Throttling for progressive rendering
                let lastProgressRender = 0;

                // Handle worker messages
                worker.onmessage = (e) => {
                    const { type, solution, score, stats, message, stack, iteration, totalIterations } = e.data;

                    switch (type) {
                        case 'progress':
                            lastProgressRender = this._handleProgressMessage(e.data, transitionLinks, nodes, lastProgressRender);
                            break;

                        case 'intermediate_solution':
                            // Progressive refinement: apply intermediate solution without completing
                            console.log(`[OPTIMIZE PROGRESSIVE] Intermediate solution (iteration ${iteration}/${totalIterations}): score=${score.toFixed(1)}`);
                            this.optimizer._applySolutionToLinks(solution, transitionLinks, nodes);

                            // Trigger visualization update via progress callback
                            if (onProgress) {
                                onProgress(iteration, totalIterations, score);
                            }
                            break;

                        case 'solution':
                            worker = this._handleSolutionMessage(solution, score, stats, transitionLinks, nodes, worker, onComplete);
                            break;

                        case 'cancelled':
                            worker = this._handleCancelledMessage(worker, onComplete);
                            break;

                        case 'failed':
                            worker = this._handleFailedMessage(worker, onComplete);
                            break;

                        case 'error':
                            worker = this._handleErrorMessage(message, stack, worker, onComplete);
                            break;
                    }
                };

                worker.onerror = (error) => {
                    console.error('[OPTIMIZE PROGRESSIVE] Worker error:', error);
                    worker.terminate();
                    worker = null;
                    if (onComplete) onComplete(false);
                };

                // Convert greedy results to CSP solution format
                const greedySolution = this.optimizer.convertGreedyToCSPSolution(transitionLinks);

                // Build parent-child map
                const parentChildMap = {};
                this.optimizer.links.forEach(link => {
                    if (link.linkType === 'containment') {
                        parentChildMap[link.target] = link.source;
                    }
                });

                // Serialize greedySolution for Worker (Map → Object)
                const greedySolutionSerialized = {
                    assignment: Object.fromEntries(greedySolution.assignment),
                    score: greedySolution.score,
                    preferences: Object.fromEntries(greedySolution.preferences)
                };

                console.log(`[OPTIMIZE PROGRESSIVE] Sending greedy solution to Worker: score=${greedySolution.score.toFixed(1)}`);

                // Send solve request to worker with greedy warm-start and dragged node
                worker.postMessage({
                    type: 'solve',
                    links: transitionLinks,
                    nodes: nodes,
                    greedySolution: greedySolutionSerialized,
                    parentChildMap: parentChildMap,
                    draggedNodeId: draggedNodeId
                });

            } catch (error) {
                console.error('[OPTIMIZE PROGRESSIVE] Failed to create Worker:', error);
                console.warn('[OPTIMIZE PROGRESSIVE] Falling back to main thread CSP');
                this.fallbackToMainThreadCSP(transitionLinks, nodes, draggedNodeId, onComplete);
            }
        }, debounceMs);

        return { cancel };
    }

    fallbackToMainThreadCSP(links, nodes, draggedNodeId, onComplete) {
        // Note: Expects already filtered links (transition and initial only)

        try {
            // Check if ConstraintSolver is available
            if (typeof ConstraintSolver === 'undefined') {
                console.warn('[OPTIMIZE PROGRESSIVE] ConstraintSolver not found, skipping CSP');
                if (onComplete) onComplete(false);
                return;
            }

            console.log('[OPTIMIZE PROGRESSIVE] Starting progressive CSP refinement (8 iterations × 250ms, main thread)...');

            // Build parent-child map
            const parentChildMap = new Map();
            this.optimizer.links.forEach(link => {
                if (link.linkType === 'containment') {
                    parentChildMap.set(link.target, link.source);
                }
            });

            // Convert greedy results to CSP solution format
            let currentBestSolution = this.optimizer.convertGreedyToCSPSolution(links);
            console.log(`[OPTIMIZE PROGRESSIVE] Initial solution: score=${currentBestSolution.score.toFixed(1)}`);

            // Progressive refinement: 8 iterations × 250ms = 2000ms total
            const MAX_ITERATIONS = 8;
            let bestScoreOverall = currentBestSolution.score;
            let bestSolutionOverall = null;

            for (let iteration = 0; iteration < MAX_ITERATIONS; iteration++) {
                console.log(`[OPTIMIZE PROGRESSIVE] === Iteration ${iteration + 1}/${MAX_ITERATIONS} ===`);

                // Create CSP solver with current best solution as warm-start
                const solver = new ConstraintSolver(links, nodes, this.optimizer, parentChildMap, currentBestSolution, draggedNodeId);

                // Run CSP for 250ms (blocks main thread)
                const solution = solver.solve();

                if (solver.cancelled) {
                    console.log(`[OPTIMIZE PROGRESSIVE] CSP cancelled during iteration ${iteration + 1}`);
                    if (onComplete) onComplete(false);
                    return;
                }

                // Update best solution if improved
                if (solution && solver.bestScore < bestScoreOverall) {
                    bestScoreOverall = solver.bestScore;
                    bestSolutionOverall = solution;

                    console.log(`[OPTIMIZE PROGRESSIVE] Iteration ${iteration + 1}: New best score=${bestScoreOverall.toFixed(1)}`);

                    // Convert solution to warm-start format for next iteration
                    currentBestSolution = {
                        assignment: solution,
                        score: solver.bestScore,
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
                    console.log(`[OPTIMIZE PROGRESSIVE] Iteration ${iteration + 1}: No improvement (current best: ${bestScoreOverall.toFixed(1)})`);
                }

                // Apply intermediate solution EVERY iteration (even if no improvement)
                if (bestSolutionOverall) {
                    console.log(`[OPTIMIZE PROGRESSIVE] Applying intermediate solution (iteration ${iteration + 1}/${MAX_ITERATIONS})...`);
                    bestSolutionOverall.forEach((assignment, linkId) => {
                        const link = links.find(l => l.id === linkId);
                        if (link) {
                            link.routing = RoutingState.fromEdges(
                                assignment.sourceEdge,
                                assignment.targetEdge
                            );
                        }
                    });

                    // Distribute snap points for intermediate solution
                    const sortedLinks = [...links].sort((a, b) => {
                        if (a.linkType === 'initial' && b.linkType !== 'initial') return -1;
                        if (a.linkType !== 'initial' && b.linkType === 'initial') return 1;
                        return 0;
                    });
                    this.optimizer.distributeSnapPointsOnEdges(sortedLinks, nodes);

                    // Invoke onProgress callback
                    if (onProgress) {
                        onProgress(iteration + 1, MAX_ITERATIONS, bestScoreOverall);
                    }
                }
            }

            // Apply final best solution
            if (bestSolutionOverall) {
                console.log(`[OPTIMIZE PROGRESSIVE] All iterations complete. Final solution: score=${bestScoreOverall.toFixed(1)}`);
                console.log('[OPTIMIZE PROGRESSIVE] Main thread CSP complete!');
                if (onComplete) onComplete(true);
            } else {
                console.log('[OPTIMIZE PROGRESSIVE] No solution found after all iterations');
                if (onComplete) onComplete(false);
            }

        } catch (error) {
            console.error('[OPTIMIZE PROGRESSIVE] Main thread CSP error:', error);
            if (onComplete) onComplete(false);
        }
    }

    optimizeSnapPointAssignments(links, nodes, useGreedy = false, draggedNodeId = null) {
        // Filter out containment and delegation links (only optimize transition and initial links)
        // Visualizer layout: containment is hierarchical structure, not routing path
        const transitionLinks = links.filter(link => 
            link.linkType === 'transition' || link.linkType === 'initial'
        );

        // Adaptive Algorithm Selection:
        // - useGreedy=true: Fast greedy algorithm for real-time drag (1-5ms)
        // - useGreedy=false: Optimal CSP solver for final result (50-200ms)

        // Complexity threshold: CSP is O(16^n), greedy is O(n^2)
        // For n > 15, CSP becomes too slow (>1000ms), use greedy instead

        if (useGreedy) {
            console.log('[OPTIMIZE] Using fast greedy algorithm (drag mode)...');
            this.optimizer.optimizeSnapPointAssignmentsGreedy(transitionLinks, nodes, draggedNodeId);
            return;
        }

        // Progressive Enhancement Strategy:
        // 1. First: Apply Greedy algorithm immediately (fast, O(n))
        // 2. Then: Run CSP solver in background (slow, optimal)
        // 3. Update layout if CSP finds better solution

        console.log(`[OPTIMIZE] Progressive optimization for ${transitionLinks.length} transitions...`);
        console.log('[OPTIMIZE GREEDY] Applying Greedy algorithm immediately...');

        // Apply Greedy first for instant feedback
        this.optimizer.optimizeSnapPointAssignmentsGreedy(transitionLinks, nodes, draggedNodeId);
        
        console.log('[OPTIMIZE GREEDY] Greedy layout applied');

        // Skip CSP if too many transitions
        if (transitionLinks.length > TransitionLayoutOptimizer.CSP_THRESHOLD) {
            console.log(`[OPTIMIZE] ${transitionLinks.length} > ${TransitionLayoutOptimizer.CSP_THRESHOLD}, skipping CSP optimization`);
            return;
        }

        // Check if ConstraintSolver is available
        if (typeof ConstraintSolver === 'undefined') {
            console.warn('[OPTIMIZE] ConstraintSolver not found, skipping CSP optimization');
            return;
        }

        // Run CSP solver asynchronously in background
        // Check if CSP is already running to avoid concurrent executions
        if (this.optimizer.cspRunning) {
            console.log('[OPTIMIZE CSP] CSP already running, skipping duplicate optimization');
            return;
        }

        console.log('[OPTIMIZE CSP] Starting background CSP optimization...');
        
        // Set flag IMMEDIATELY to prevent duplicate scheduling
        // (requestIdleCallback has delay between schedule and execution)
        this.optimizer.cspRunning = true;
        
        // Use requestIdleCallback for truly non-blocking execution
        // CSP will only run when browser is idle (not blocking user interactions)
        const scheduleCSP = () => {
            if (typeof requestIdleCallback !== 'undefined') {
                requestIdleCallback(() => {
                    this.optimizer.runBackgroundCSPOptimization(transitionLinks, nodes, draggedNodeId);
                }, { timeout: 5000 }); // Fallback to setTimeout after 5s
            } else {
                // Fallback for browsers without requestIdleCallback
                setTimeout(() => {
                    this.optimizer.runBackgroundCSPOptimization(transitionLinks, nodes, draggedNodeId);
                }, 100); // Small delay to let UI render
            }
        };
        
        scheduleCSP();
    }

    convertGreedyToCSPSolution(links) {
        const assignment = new Map();
        const preferences = new Map();
        let totalScore = 0;

        links.forEach(link => {
            if (!link.routing) return;

            const sourceNode = this.optimizer.nodes.find(n => n.id === this.getVisualSource(link));
            const targetNode = this.optimizer.nodes.find(n => n.id === this.getVisualTarget(link));
            if (!sourceNode || !targetNode) return;

            const combo = {
                sourceEdge: link.routing.sourceEdge,
                targetEdge: link.routing.targetEdge,
                sourcePoint: this.optimizer.getEdgeCenterPoint(sourceNode, link.routing.sourceEdge),
                targetPoint: this.optimizer.getEdgeCenterPoint(targetNode, link.routing.targetEdge)
            };

            const dx = combo.targetPoint.x - combo.sourcePoint.x;
            const dy = combo.targetPoint.y - combo.sourcePoint.y;
            combo.distance = Math.sqrt(dx * dx + dy * dy);

            // Calculate score (use same logic as greedy for consistency)
            const score = this.optimizer.calculateComboScore(link, combo, assignment, links);

            assignment.set(link.id, {
                sourceEdge: link.routing.sourceEdge,
                targetEdge: link.routing.targetEdge,
                combo: combo,
                score: score
            });

            preferences.set(link.id, {
                sourceEdge: link.routing.sourceEdge,
                targetEdge: link.routing.targetEdge
            });

            totalScore += score;
        });

        return { assignment, score: totalScore, preferences };
    }

    runBackgroundCSPOptimization(transitionLinks, nodes, draggedNodeId) {
        // Note: cspRunning flag already set in optimizeSnapPointAssignments()
        // to prevent duplicate scheduling before requestIdleCallback executes
        
        console.log('[CSP BACKGROUND] Starting CSP solver (non-blocking, max 500ms)...');
        const startTime = performance.now();

        // Build parent-child map from containment links
        const parentChildMap = new Map(); // Map<childId, parentId>
        this.optimizer.links.forEach(link => {
            if (link.linkType === 'containment') {
                parentChildMap.set(link.target, link.source);
            }
        });
        
        console.log(`[CSP BACKGROUND] Built parent-child map with ${parentChildMap.size} entries`);
        for (const [child, parent] of parentChildMap.entries()) {
            console.log(`  ${parent} → ${child}`);
        }

        // Convert greedy results to CSP solution format (Strategy 1 + 3)
        const greedySolution = this.optimizer.convertGreedyToCSPSolution(transitionLinks);
        console.log(`[CSP BACKGROUND] Converted greedy solution: score=${greedySolution.score.toFixed(1)}, ${greedySolution.preferences.size} link preferences`);

        // Backup greedy routing for potential restore
        const previousRoutings = new Map();
        transitionLinks.forEach(link => {
            if (link.routing) {
                previousRoutings.set(link.id, link.routing);
            }
        });

        // Clear routing before CSP (CSP will use greedy as initial solution, not direct routing)
        transitionLinks.forEach(link => {
            link.routing = null;
        });

        // Create CSP solver with greedy warm-start and dragged node locality
        const solver = new ConstraintSolver(transitionLinks, nodes, this.optimizer, parentChildMap, greedySolution, draggedNodeId);

        // Solve
        const solution = solver.solve();
        const elapsedMs = performance.now() - startTime;

        if (!solution) {
            console.warn(`[CSP BACKGROUND] No solution found after ${elapsedMs.toFixed(1)}ms, keeping Greedy layout`);
            // Restore previous routing
            transitionLinks.forEach(link => {
                if (previousRoutings.has(link.id)) {
                    link.routing = previousRoutings.get(link.id);
                }
            });
            // Clear flag
            this.optimizer.cspRunning = false;
            return;
        }

        console.log(`[CSP BACKGROUND] Solution found after ${elapsedMs.toFixed(1)}ms, updating layout...`);

        // Apply solution to links
        solution.forEach((assignment, linkId) => {
            const link = transitionLinks.find(l => l.id === linkId);
            if (link) {
                link.routing = RoutingState.fromEdges(
                    assignment.sourceEdge,
                    assignment.targetEdge
                );

                console.log(`[CSP BACKGROUND UPDATE] ${this.getVisualSource(link)}→${this.getVisualTarget(link)}: ${assignment.sourceEdge}→${assignment.targetEdge}`);
            }
        });

        // Distribute snap points on each edge to minimize congestion
        const sortedLinks = [...transitionLinks].sort((a, b) => {
            if (a.linkType === 'initial' && b.linkType !== 'initial') return -1;
            if (a.linkType !== 'initial' && b.linkType === 'initial') return 1;
            return 0;
        });
        this.optimizer.distributeSnapPointsOnEdges(sortedLinks, nodes);

        console.log('[CSP BACKGROUND] Layout updated with CSP solution');
        
        // Trigger re-render if callback exists
        if (this.optimizer.onLayoutUpdated) {
            console.log('[CSP BACKGROUND] Triggering re-render...');
            this.optimizer.onLayoutUpdated();
        }

        // Clear flag
        this.optimizer.cspRunning = false;
    }

    optimizeSnapPointAssignmentsGreedy(links, nodes, draggedNodeId = null) {
        console.log(`[OPTIMIZE GREEDY] Input: ${links.length} links, ${nodes.length} nodes`);
        console.log(`[OPTIMIZE GREEDY] Node IDs: ${nodes.map(n => n.id).join(', ')}`);
        console.log('[OPTIMIZE GREEDY] Using greedy algorithm (fallback)...');

        // OPTIMIZATION: Cache CSP routing for links unaffected by drag
        // Distance threshold: links within this distance are affected by drag
        const DRAG_IMPACT_RADIUS = 300; // pixels

        const assignedPaths = [];
        let cachedCount = 0;
        let recalculatedCount = 0;

        // Process initial transitions first to establish edge blocking
        const sortedLinks = [...links].sort((a, b) => {
            if (a.linkType === 'initial' && b.linkType !== 'initial') return -1;
            if (a.linkType !== 'initial' && b.linkType === 'initial') return 1;
            return 0;
        });

        // Build map of reverse links
        const reverseLinkMap = new Map();
        links.forEach(link => {
            const key = `${this.getVisualSource(link)}→${this.getVisualTarget(link)}`;
            reverseLinkMap.set(key, link);
        });

        // Assign optimal edges to each link (greedy selection)
        sortedLinks.forEach(link => {
            const sourceNode = nodes.find(n => n.id === this.getVisualSource(link));
            const targetNode = nodes.find(n => n.id === this.getVisualTarget(link));

            if (!sourceNode || !targetNode) return;

            // OPTIMIZATION: Check if link is affected by drag
            let isAffectedByDrag = true; // Default: recalculate everything

            if (draggedNodeId && link.routing) {
                // Has dragged node and link has existing routing (from CSP)

                // Direct connection to dragged node?
                const isDirectlyConnected = (this.getVisualSource(link) === draggedNodeId || this.getVisualTarget(link) === draggedNodeId);

                if (!isDirectlyConnected) {
                    // Check distance to dragged node
                    const draggedNode = nodes.find(n => n.id === draggedNodeId);
                    if (draggedNode) {
                        const sourceDist = Math.hypot(sourceNode.x - draggedNode.x, sourceNode.y - draggedNode.y);
                        const targetDist = Math.hypot(targetNode.x - draggedNode.x, targetNode.y - draggedNode.y);
                        const minDist = Math.min(sourceDist, targetDist);

                        // Far enough from drag? Cache CSP result!
                        if (minDist > DRAG_IMPACT_RADIUS) {
                            isAffectedByDrag = false;
                        }
                    }
                }
            }

            // CACHE: Keep CSP routing for unaffected links
            if (!isAffectedByDrag) {
                // Reconstruct path combo from cached routing
                const cachedCombo = this.optimizer.getAllPossibleSnapCombinations(link, sourceNode, targetNode, null)
                    .find(combo => combo.sourceEdge === link.routing.sourceEdge &&
                                   combo.targetEdge === link.routing.targetEdge);

                if (cachedCombo) {
                    assignedPaths.push(cachedCombo);
                    cachedCount++;
                    console.log(`[OPTIMIZE GREEDY CACHE] ${this.getVisualSource(link)}→${this.getVisualTarget(link)}: keeping CSP routing ${link.routing.sourceEdge}→${link.routing.targetEdge}`);
                    return; // Skip recalculation
                }
            }

            // Link affected by drag: recalculate
            recalculatedCount++;

            // Check if reverse link exists and has routing assigned
            const reverseKey = `${this.getVisualTarget(link)}→${this.getVisualSource(link)}`;
            const reverseLink = reverseLinkMap.get(reverseKey);
            const reverseRouting = (reverseLink && reverseLink.routing) ? reverseLink.routing : null;

            if (reverseRouting) {
                console.log(`[BIDIRECTIONAL DETECT] ${this.getVisualSource(link)}→${this.getVisualTarget(link)} has reverse link with routing: ${reverseRouting.sourceEdge}→${reverseRouting.targetEdge}`);
            }

            // Get all possible combinations (exclude reverse edge pair if bidirectional)
            const combinations = this.optimizer.getAllPossibleSnapCombinations(link, sourceNode, targetNode, reverseRouting);

            if (combinations.length === 0) {
                console.warn(`No valid snap combinations for ${this.getVisualSource(link)}→${this.getVisualTarget(link)}`);
                return;
            }

            // OPTIMIZATION: Prioritize previous CSP routing in value ordering
            // Check combinations in order: CSP routing first, then others
            if (link.routing) {
                const previousRouting = link.routing;
                combinations.sort((a, b) => {
                    const aMatchesCSP = (a.sourceEdge === previousRouting.sourceEdge &&
                                        a.targetEdge === previousRouting.targetEdge);
                    const bMatchesCSP = (b.sourceEdge === previousRouting.sourceEdge &&
                                        b.targetEdge === previousRouting.targetEdge);

                    if (aMatchesCSP && !bMatchesCSP) return -1;
                    if (!aMatchesCSP && bMatchesCSP) return 1;
                    return 0; // Preserve original order for others
                });
            }

            // Score each combination
            let bestCombination = null;
            let bestScore = Infinity;

            combinations.forEach(combo => {
                // Calculate intersections with already assigned paths
                let intersections = 0;
                assignedPaths.forEach(assignedPath => {
                    intersections += this.optimizer.calculatePathIntersections(combo, assignedPath);
                });

                // Calculate node collisions
                let nodeCollisions = 0;

                if (this.optimizer.pathIntersectsNode(combo, sourceNode, { skipFirstSegment: true })) {
                    nodeCollisions++;
                }

                if (this.optimizer.pathIntersectsNode(combo, targetNode, { skipLastSegment: true })) {
                    nodeCollisions++;
                }

                nodes.forEach(node => {
                    if (node.id === sourceNode.id || node.id === targetNode.id) return;

                    if (this.optimizer.pathIntersectsNode(combo, node)) {
                        nodeCollisions++;
                    }
                });

                // Check if snap points are too close
                const MIN_SAFE_DISTANCE = TransitionLayoutOptimizer.MIN_SAFE_DISTANCE;
                let tooCloseSnap = 0;

                const sourceEdge = combo.sourceEdge;
                const targetEdge = combo.targetEdge;
                const sourceIsVertical = (sourceEdge === 'top' || sourceEdge === 'bottom');
                const targetIsVertical = (targetEdge === 'top' || targetEdge === 'bottom');

                const dx = Math.abs(combo.sourcePoint.x - combo.targetPoint.x);
                const dy = Math.abs(combo.sourcePoint.y - combo.targetPoint.y);

                if (!sourceIsVertical && !targetIsVertical) {
                    if (dx < MIN_SAFE_DISTANCE && dy < MIN_SAFE_DISTANCE) {
                        tooCloseSnap = 1;
                    }
                } else if (sourceIsVertical && targetIsVertical) {
                    if (dy < MIN_SAFE_DISTANCE && dx < MIN_SAFE_DISTANCE) {
                        tooCloseSnap = 1;
                    }
                }

                // Check for MIN_SEGMENT self-overlap
                const MIN_SEGMENT = TransitionLayoutOptimizer.MIN_SEGMENT_LENGTH;
                let selfOverlap = 0;

                // Case 1: Both horizontal edges (H-V-H path)
                if (!sourceIsVertical && !targetIsVertical) {
                    // Calculate intermediate points
                    let x1; // After source MIN_SEGMENT
                    if (sourceEdge === 'right') {
                        x1 = combo.sourcePoint.x + MIN_SEGMENT;
                    } else { // left
                        x1 = combo.sourcePoint.x - MIN_SEGMENT;
                    }

                    let x2; // Before target MIN_SEGMENT
                    if (targetEdge === 'right') {
                        x2 = combo.targetPoint.x + MIN_SEGMENT;
                    } else { // left
                        x2 = combo.targetPoint.x - MIN_SEGMENT;
                    }

                    // Detect overlap: x1 should not be between x2 and tx
                    if (targetEdge === 'left') {
                        // Target MIN_SEGMENT: x2 ← tx
                        // x1 must be left of x2 to avoid overlap
                        if (x1 > x2) {
                            selfOverlap = 1;
                        }
                    } else { // right
                        // Target MIN_SEGMENT: tx → x2
                        // x1 must be right of x2 to avoid overlap
                        if (x1 < x2) {
                            selfOverlap = 1;
                        }
                    }
                }

                // Case 2: Both vertical edges (V-H-V path)
                if (sourceIsVertical && targetIsVertical) {
                    // Calculate intermediate points
                    let y1; // After source MIN_SEGMENT
                    if (sourceEdge === 'top') {
                        y1 = combo.sourcePoint.y - MIN_SEGMENT;
                    } else { // bottom
                        y1 = combo.sourcePoint.y + MIN_SEGMENT;
                    }

                    let y2; // Before target MIN_SEGMENT
                    if (targetEdge === 'top') {
                        y2 = combo.targetPoint.y - MIN_SEGMENT;
                    } else { // bottom
                        y2 = combo.targetPoint.y + MIN_SEGMENT;
                    }

                    // Detect overlap: y1 should not be between y2 and ty
                    if (targetEdge === 'top') {
                        // Target MIN_SEGMENT: y2 ← ty (going up)
                        // y1 must be above y2 to avoid overlap
                        if (y1 > y2) {
                            selfOverlap = 1;
                        }
                    } else { // bottom
                        // Target MIN_SEGMENT: ty → y2 (going down)
                        // y1 must be below y2 to avoid overlap
                        if (y1 < y2) {
                            selfOverlap = 1;
                        }
                    }
                }

                // Case 3: Mixed edges (V-H or H-V)
                // No self-overlap possible: MIN_SEGMENTs operate on orthogonal axes

                // Score with penalty weights (selfOverlap weight: 50000, same as CSP solver)
                const score = tooCloseSnap * 100000 + selfOverlap * 50000 + nodeCollisions * 10000 + intersections * 1000 + combo.distance;

                if (score < bestScore) {
                    bestScore = score;
                    bestCombination = combo;
                }
            });

            // Assign best edge combination
            if (bestCombination) {
                link.routing = RoutingState.fromEdges(
                    bestCombination.sourceEdge,
                    bestCombination.targetEdge
                );

                assignedPaths.push(bestCombination);

                console.log(`[OPTIMIZE GREEDY] ${this.getVisualSource(link)}→${this.getVisualTarget(link)}: ${bestCombination.sourceEdge}→${bestCombination.targetEdge}, score=${bestScore.toFixed(1)}`);
            }
        });

        // Distribute snap points on each edge to minimize congestion
        this.optimizer.distributeSnapPointsOnEdges(sortedLinks, nodes);

        console.log(`[OPTIMIZE GREEDY] Completed: ${links.length} links (cached: ${cachedCount}, recalculated: ${recalculatedCount})`);
    }
}
