// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * ConstraintSolver
 *
 * Constraint Satisfaction Problem (CSP) solver for optimal transition routing.
 * Uses backtracking search with constraint propagation to find globally optimal
 * edge assignments that minimize crossing and maximize visual clarity.
 *
 * Key Features:
 * 1. Snap Point Simulation
 *    Instead of using edge center points for intersection detection (which caused
 *    Phase 1/Phase 2 mismatch), this solver simulates the actual snap point positions
 *    that will be distributed in Phase 2 when multiple transitions share an edge.
 *    This ensures accurate crossing detection during CSP solving.
 *
 * 2. Performance Optimizations
 *    - O(1) Lookup Maps: nodeMap, linkMap for instant access (eliminates O(n) find)
 *    - Edge Usage Tracking: edgeUsage Map for O(1) edge sharing queries
 *    - Snap Point Memoization: Cache simulation results (auto-invalidated by assignmentKey in cache key)
 *    - Stored Scores: Pre-calculated scores in assignments (O(1) lookup in early pruning and base case)
 *    - Partial Score Caching: Calculate currentPartialScore once per depth (O(m) once vs O(m × domain_size))
 *    - On-demand Scoring: Calculate soft constraints only when trying a combo (skip unused branches)
 *    - Early Termination: Stop search when perfect solution found (score=0) and return immediately on first intersection
 *    - Zero Cache Invalidation: assignmentKey in cache key auto-handles state changes (eliminates O(cache_size) × 4 overhead)
 *    - Console.log disabled: Zero overhead in production (controlled by DEBUG_CSP_SOLVER flag)
 */

// Debug flag - set to true to enable detailed logging (WARNING: extremely slow)
const DEBUG_CSP_SOLVER = false;

// No-op logging function for production
const log = DEBUG_CSP_SOLVER ? console.log.bind(console) : () => {};

/**
 * HardConstraints - Must be satisfied (Boolean: Pass/Fail)
 */
class HardConstraints {
    /**
     * HC1: No initial edge blocking
     * Initial transition이 사용하는 edge는 다른 transition 사용 불가
     */
    static validateInitialBlocking(link, sourceEdge, targetEdge, optimizer) {
        if (link.linkType === 'initial') return true;

        if (optimizer.hasInitialTransitionOnEdge(link.source, sourceEdge)) {
            return false; // FAIL: Source edge blocked
        }
        if (optimizer.hasInitialTransitionOnEdge(link.target, targetEdge)) {
            return false; // FAIL: Target edge blocked
        }
        return true; // PASS
    }

    /**
     * HC2: No node collisions
     * Path가 source/target 이외의 노드를 관통하지 않음
     */
    static validateNodeCollisions(combo, sourceNode, targetNode, allNodes, optimizer) {
        // Source node collision (skip first segment)
        if (optimizer.pathIntersectsNode(combo, sourceNode, { skipFirstSegment: true })) {
            return false; // FAIL
        }

        // Target node collision (skip last segment)
        if (optimizer.pathIntersectsNode(combo, targetNode, { skipLastSegment: true })) {
            return false; // FAIL
        }

        // Other nodes collision
        for (const node of allNodes) {
            if (node.id === sourceNode.id || node.id === targetNode.id) continue;
            if (optimizer.pathIntersectsNode(combo, node)) {
                return false; // FAIL
            }
        }

        return true; // PASS
    }

    /**
     * HC3: No too-close snaps
     * Snap point가 너무 가까우면 path가 target node와 충돌
     */
    static validateMinimumDistance(combo) {
        const MIN_SAFE_DISTANCE = TransitionLayoutOptimizer.MIN_SAFE_DISTANCE;
        const sourceIsVert = (combo.sourceEdge === 'top' || combo.sourceEdge === 'bottom');
        const targetIsVert = (combo.targetEdge === 'top' || combo.targetEdge === 'bottom');

        const dx = Math.abs(combo.sourcePoint.x - combo.targetPoint.x);
        const dy = Math.abs(combo.sourcePoint.y - combo.targetPoint.y);

        if (!sourceIsVert && !targetIsVert) {
            // Both horizontal
            if (dx < MIN_SAFE_DISTANCE && dy < MIN_SAFE_DISTANCE) {
                return false; // FAIL
            }
        } else if (sourceIsVert && targetIsVert) {
            // Both vertical
            if (dy < MIN_SAFE_DISTANCE && dx < MIN_SAFE_DISTANCE) {
                return false; // FAIL
            }
        }

        return true; // PASS
    }

    /**
     * HC4: No bidirectional edge pair conflicts
     * 양방향 링크는 같은 edge pair를 반대 방향으로 사용할 수 없음
     */
    static validateBidirectionalConflict(link, sourceEdge, targetEdge, assignment, reverseLinkMap) {
        const reverseKey = `${link.target}→${link.source}`;
        const reverseLink = reverseLinkMap.get(reverseKey);

        if (!reverseLink || !assignment.has(reverseLink.id)) {
            return true; // No conflict
        }

        const reverseAssignment = assignment.get(reverseLink.id);

        // Check if current assignment is reverse of the assigned routing
        if (sourceEdge === reverseAssignment.targetEdge &&
            targetEdge === reverseAssignment.sourceEdge) {
            return false; // FAIL: Conflict
        }

        return true; // PASS
    }

    /**
     * Validate all hard constraints
     */
    static validateAll(link, sourceEdge, targetEdge, combo, sourceNode, targetNode,
                       allNodes, assignment, reverseLinkMap, optimizer) {
        return (
            this.validateInitialBlocking(link, sourceEdge, targetEdge, optimizer) &&
            this.validateNodeCollisions(combo, sourceNode, targetNode, allNodes, optimizer) &&
            this.validateMinimumDistance(combo) &&
            this.validateBidirectionalConflict(link, sourceEdge, targetEdge, assignment, reverseLinkMap)
        );
    }
}

/**
 * SoftConstraints - Weighted scoring (minimize/maximize)
 */
class SoftConstraints {
    // Memoization cache for snap point calculations
    // Cache key: "linkId:nodeId:edge:isSource:assignmentKey"
    // assignmentKey: sorted comma-separated list of link IDs using the same edge
    static snapPointCache = new Map();

    /**
     * Clear entire cache (used when solver restarts)
     */
    static clearCache() {
        this.snapPointCache.clear();
    }
    /**
     * SC1: Minimize intersections
     * Weight: 50000 (highest priority soft constraint)
     * Uses snap point simulation for accurate intersection detection
     */
    static scoreIntersections(link, combo, assignment, solver) {
        // Simulate snap points for current combo considering existing assignments
        const simulatedCombo = this.simulateSnapPoints(link, combo, assignment, solver);

        for (const [assignedLinkId, assignedData] of assignment) {
            const assignedLink = solver.linkMap.get(assignedLinkId);
            if (!assignedLink) continue;

            // Simulate snap points for assigned path as well
            const simulatedAssigned = this.simulateSnapPoints(
                assignedLink,
                assignedData.combo,
                assignment,
                solver
            );

            const intersections = solver.optimizer.calculatePathIntersections(simulatedCombo, simulatedAssigned);
            
            // Performance Optimization: Return immediately if intersection found
            if (intersections > 0) {
                return 50000; // Penalty for any intersection, no need to count more
            }
        }

        return 0; // No intersections
    }

    /**
     * Simulate snap point distribution for a link based on current assignments
     * Matches Phase 2 logic in TransitionLayoutOptimizer.distributeSnapPointsOnEdges()
     */
    static simulateSnapPoints(link, combo, assignment, solver) {
        const sourceNode = solver.nodeMap.get(link.source);
        const targetNode = solver.nodeMap.get(link.target);

        if (!sourceNode || !targetNode) return combo;

        // Simulate source edge snap point
        const sourcePoint = this.simulateEdgeSnapPoint(
            link, sourceNode, combo.sourceEdge, true, assignment, solver
        );

        // Simulate target edge snap point
        const targetPoint = this.simulateEdgeSnapPoint(
            link, targetNode, combo.targetEdge, false, assignment, solver
        );

        return {
            ...combo,
            sourcePoint,
            targetPoint
        };
    }

    /**
     * Simulate snap point on a specific edge
     */
    static simulateEdgeSnapPoint(link, node, edge, isSource, assignment, solver) {
        // Initial transitions use center point
        if (link.linkType === 'initial' && isSource) {
            return { x: node.x || 0, y: node.y || 0 };
        }

        // Generate cache key based on link, edge, and current assignment state
        const edgeLinkIds = solver.getEdgeLinks(node.id, edge);
        const assignmentKey = Array.from(edgeLinkIds).sort().join(',');
        const cacheKey = `${link.id}:${node.id}:${edge}:${isSource}:${assignmentKey}`;

        // Check cache
        if (this.snapPointCache.has(cacheKey)) {
            return this.snapPointCache.get(cacheKey);
        }

        // Collect all links that will use this edge (including current link)
        const edgeLinks = [];

        // Add current link
        edgeLinks.push({ link, isSource, isCurrentLink: true });

        // Performance Optimization: Use edge usage tracking for O(1) lookup (reuse edgeLinkIds from cache key)
        for (const linkId of edgeLinkIds) {
            if (linkId === link.id) continue; // Skip current link (already added)

            const assignedLink = solver.linkMap.get(linkId);
            const assignedData = assignment.get(linkId);
            if (!assignedLink || !assignedData) continue;

            // Determine if this link uses the edge as source or target
            if (assignedLink.source === node.id && assignedData.sourceEdge === edge) {
                edgeLinks.push({ link: assignedLink, isSource: true, isCurrentLink: false });
            } else if (assignedLink.target === node.id && assignedData.targetEdge === edge) {
                edgeLinks.push({ link: assignedLink, isSource: false, isCurrentLink: false });
            }
        }

        if (edgeLinks.length === 1) {
            // Single link on edge - use center point
            return solver.optimizer.getEdgeCenterPoint(node, edge);
        }

        // Separate incoming and outgoing
        const incoming = edgeLinks.filter(item => !item.isSource);
        const outgoing = edgeLinks.filter(item => item.isSource);

        // Sort by other node position
        const sortByOtherNode = (a, b) => {
            const aOther = solver.nodeMap.get(a.isSource ? a.link.target : a.link.source);
            const bOther = solver.nodeMap.get(b.isSource ? b.link.target : b.link.source);

            if (!aOther || !bOther) return 0;

            if (edge === 'top' || edge === 'bottom') {
                return (aOther.x || 0) - (bOther.x || 0);
            } else {
                return (aOther.y || 0) - (bOther.y || 0);
            }
        };

        incoming.sort(sortByOtherNode);
        outgoing.sort(sortByOtherNode);

        // Combine: incoming first, then outgoing (matches Phase 2)
        const sortedLinks = [...incoming, ...outgoing];
        const totalCount = sortedLinks.length;

        // Find index of current link
        const currentIndex = sortedLinks.findIndex(item => item.isCurrentLink);
        if (currentIndex < 0) {
            console.error('[SIMULATE ERROR] Current link not found in sorted list');
            return optimizer.getEdgeCenterPoint(node, edge);
        }

        // Calculate position (matches Phase 2 distribution)
        const position = (currentIndex + 1) / (totalCount + 1);

        const cx = node.x || 0;
        const cy = node.y || 0;
        const { halfWidth, halfHeight } = TransitionLayoutOptimizer.getNodeSize(node.type);

        let x, y;

        if (edge === 'top') {
            x = cx - halfWidth + (halfWidth * 2 * position);
            y = cy - halfHeight;
        } else if (edge === 'bottom') {
            x = cx - halfWidth + (halfWidth * 2 * position);
            y = cy + halfHeight;
        } else if (edge === 'left') {
            x = cx - halfWidth;
            y = cy - halfHeight + (halfHeight * 2 * position);
        } else if (edge === 'right') {
            x = cx + halfWidth;
            y = cy - halfHeight + (halfHeight * 2 * position);
        }

        log(`[SIMULATE SNAP] ${link.source}→${link.target} ${isSource ? 'source' : 'target'} on ${node.id}.${edge}: position ${currentIndex + 1}/${totalCount} at (${x.toFixed(1)}, ${y.toFixed(1)})`);

        const result = { x, y };
        
        // Store in cache
        this.snapPointCache.set(cacheKey, result);
        
        return result;
    }

    /**
     * SC2: Minimize distance
     * Weight: 1 (lowest priority - tiebreaker)
     */
    static scoreDistance(combo) {
        return combo.distance;
    }

    /**
     * SC3: Prefer symmetry for bidirectional pairs
     * Bonus: -5000 (negative = reward)
     */
    static scoreSymmetry(link, sourceEdge, targetEdge, assignment, reverseLinkMap) {
        const reverseKey = `${link.target}→${link.source}`;
        const reverseLink = reverseLinkMap.get(reverseKey);

        if (!reverseLink || !assignment.has(reverseLink.id)) {
            return 0; // No symmetry bonus
        }

        const reverseAssignment = assignment.get(reverseLink.id);

        // Pattern 1: Same edge (e.g., right→right + right→right)
        if (sourceEdge === targetEdge &&
            reverseAssignment.sourceEdge === reverseAssignment.targetEdge &&
            sourceEdge === reverseAssignment.sourceEdge) {
            return -5000; // Symmetric same-edge bonus
        }

        // Pattern 2: Facing edges (e.g., right→left + left→right)
        if (sourceEdge === reverseAssignment.targetEdge &&
            targetEdge === reverseAssignment.sourceEdge) {
            return -5000; // Symmetric facing-edge bonus
        }

        return 0; // No symmetry
    }

    /**
     * SC4: Detect self-overlapping segments within same transition
     * Weight: 50000 (same as intersection - critical constraint)
     * 
     * Problem: When MIN_SEGMENT ranges overlap, path backtracks on itself
     * Example: right→left with x1=460, x2=433
     *   - Source MIN_SEGMENT: 430→460
     *   - Target MIN_SEGMENT: 433→463
     *   - Path: 460 ← 433 → 463 (backtracking!)
     * 
     * Solution: Detect when intermediate point falls within target MIN_SEGMENT
     */
    static scoreSelfOverlap(combo) {
        const MIN_SEGMENT = TransitionLayoutOptimizer.MIN_SEGMENT_LENGTH;
        const sx = combo.sourcePoint.x;
        const sy = combo.sourcePoint.y;
        const tx = combo.targetPoint.x;
        const ty = combo.targetPoint.y;
        const sourceEdge = combo.sourceEdge;
        const targetEdge = combo.targetEdge;

        const sourceIsVertical = (sourceEdge === 'top' || sourceEdge === 'bottom');
        const targetIsVertical = (targetEdge === 'top' || targetEdge === 'bottom');

        // Case 1: Both horizontal edges (H-V-H path)
        if (!sourceIsVertical && !targetIsVertical) {
            // Calculate intermediate points
            let x1; // After source MIN_SEGMENT
            if (sourceEdge === 'right') {
                x1 = sx + MIN_SEGMENT;
            } else { // left
                x1 = sx - MIN_SEGMENT;
            }

            let x2; // Before target MIN_SEGMENT
            if (targetEdge === 'right') {
                x2 = tx + MIN_SEGMENT;
            } else { // left
                x2 = tx - MIN_SEGMENT;
            }

            // Detect overlap: x1 should not be between x2 and tx
            // For left target: x1 should be <= x2 (left of MIN_SEGMENT start)
            // For right target: x1 should be >= x2 (right of MIN_SEGMENT start)
            if (targetEdge === 'left') {
                // Target MIN_SEGMENT: x2 ← tx
                // x1 must be left of x2 to avoid overlap
                if (x1 > x2) {
                    log(`[SELF-OVERLAP] ${combo.sourceEdge}→${combo.targetEdge}: x1=${x1.toFixed(1)} > x2=${x2.toFixed(1)} (overlap!)`);
                    return 50000; // High penalty for self-overlap
                }
            } else { // right
                // Target MIN_SEGMENT: tx → x2
                // x1 must be right of x2 to avoid overlap
                if (x1 < x2) {
                    log(`[SELF-OVERLAP] ${combo.sourceEdge}→${combo.targetEdge}: x1=${x1.toFixed(1)} < x2=${x2.toFixed(1)} (overlap!)`);
                    return 50000;
                }
            }
        }

        // Case 2: Both vertical edges (V-H-V path)
        if (sourceIsVertical && targetIsVertical) {
            // Calculate intermediate points
            let y1; // After source MIN_SEGMENT
            if (sourceEdge === 'top') {
                y1 = sy - MIN_SEGMENT;
            } else { // bottom
                y1 = sy + MIN_SEGMENT;
            }

            let y2; // Before target MIN_SEGMENT
            if (targetEdge === 'top') {
                y2 = ty - MIN_SEGMENT;
            } else { // bottom
                y2 = ty + MIN_SEGMENT;
            }

            // Detect overlap: y1 should not be between y2 and ty
            // For top target: y1 should be <= y2 (above MIN_SEGMENT start)
            // For bottom target: y1 should be >= y2 (below MIN_SEGMENT start)
            if (targetEdge === 'top') {
                // Target MIN_SEGMENT: y2 ← ty (going up)
                // y1 must be above y2 to avoid overlap
                if (y1 > y2) {
                    log(`[SELF-OVERLAP] ${combo.sourceEdge}→${combo.targetEdge}: y1=${y1.toFixed(1)} > y2=${y2.toFixed(1)} (overlap!)`);
                    return 50000;
                }
            } else { // bottom
                // Target MIN_SEGMENT: ty → y2 (going down)
                // y1 must be below y2 to avoid overlap
                if (y1 < y2) {
                    log(`[SELF-OVERLAP] ${combo.sourceEdge}→${combo.targetEdge}: y1=${y1.toFixed(1)} < y2=${y2.toFixed(1)} (overlap!)`);
                    return 50000;
                }
            }
        }

        // Case 3: Mixed edges (V-H or H-V)
        // No self-overlap possible: MIN_SEGMENTs operate on orthogonal axes
        // HC3 already validates minimum safe distance (60px) for close nodes

        return 0; // No self-overlap detected
    }

    /**
     * Calculate total soft constraint score
     */
    static calculateTotal(link, combo, assignment, reverseLinkMap, solver) {
        const intersectionScore = this.scoreIntersections(link, combo, assignment, solver);
        const distanceScore = this.scoreDistance(combo);
        const symmetryBonus = this.scoreSymmetry(link, combo.sourceEdge, combo.targetEdge,
                                                 assignment, reverseLinkMap);
        const selfOverlapScore = this.scoreSelfOverlap(combo);

        return intersectionScore + distanceScore + symmetryBonus + selfOverlapScore;
    }
}

/**
 * ConstraintSolver - Backtracking search with constraint propagation
 */
class ConstraintSolver {
    constructor(links, nodes, optimizer) {
        this.links = links;
        this.nodes = nodes;
        this.optimizer = optimizer;

        // Performance Optimization: O(1) lookup maps
        this.nodeMap = new Map(nodes.map(n => [n.id, n]));
        this.linkMap = new Map(links.map(l => [l.id, l]));

        // Build reverse link map
        this.reverseLinkMap = new Map();
        links.forEach(link => {
            const key = `${link.source}→${link.target}`;
            this.reverseLinkMap.set(key, link);
        });

        // Priority: Initial transitions first
        this.sortedLinks = [...links].sort((a, b) => {
            if (a.linkType === 'initial' && b.linkType !== 'initial') return -1;
            if (a.linkType !== 'initial' && b.linkType === 'initial') return 1;
            return 0;
        });

        // Assignment: Map<linkId, {sourceEdge, targetEdge, combo}>
        this.assignment = new Map();

        // Performance Optimization: Edge usage tracking
        // Map<"nodeId:edge", Set<linkId>>
        this.edgeUsage = new Map();

        // Best solution tracking
        this.bestAssignment = null;
        this.bestScore = Infinity;

        // Statistics
        this.nodeCount = 0;
        this.pruneCount = 0;

        // Cancellation support for background optimization
        this.cancelled = false;

        // Progress callback for incremental updates
        this.onProgressCallback = null;
        this.startTime = 0;
    }

    /**
     * Cancel ongoing CSP search (for background optimization)
     */
    cancel() {
        this.cancelled = true;
        console.log('[CSP SOLVER] Cancellation requested');
    }

    /**
     * Get links using a specific edge (O(1) lookup)
     */
    getEdgeLinks(nodeId, edge) {
        const key = `${nodeId}:${edge}`;
        return this.edgeUsage.get(key) || new Set();
    }

    /**
     * Assign edge routing to a link and update edge usage tracking
     */
    assignEdgeRouting(link, sourceEdge, targetEdge, combo) {
        // Calculate and store score for early pruning optimization
        const score = SoftConstraints.calculateTotal(
            link, combo, this.assignment,
            this.reverseLinkMap, this
        );

        // Store assignment with pre-calculated score
        this.assignment.set(link.id, { sourceEdge, targetEdge, combo, score });

        // Track source edge usage
        const sourceKey = `${link.source}:${sourceEdge}`;
        if (!this.edgeUsage.has(sourceKey)) {
            this.edgeUsage.set(sourceKey, new Set());
        }
        this.edgeUsage.get(sourceKey).add(link.id);

        // Track target edge usage
        const targetKey = `${link.target}:${targetEdge}`;
        if (!this.edgeUsage.has(targetKey)) {
            this.edgeUsage.set(targetKey, new Set());
        }
        this.edgeUsage.get(targetKey).add(link.id);

        // Cache invalidation not needed: assignmentKey in cache key automatically handles state changes
    }

    /**
     * Unassign edge routing from a link and update edge usage tracking
     */
    unassignEdgeRouting(link) {
        const assignment = this.assignment.get(link.id);
        if (!assignment) return;

        // Remove from assignment
        this.assignment.delete(link.id);

        // Remove from source edge usage
        const sourceKey = `${link.source}:${assignment.sourceEdge}`;
        const sourceSet = this.edgeUsage.get(sourceKey);
        if (sourceSet) {
            sourceSet.delete(link.id);
            if (sourceSet.size === 0) {
                this.edgeUsage.delete(sourceKey);
            }
        }

        // Remove from target edge usage
        const targetKey = `${link.target}:${assignment.targetEdge}`;
        const targetSet = this.edgeUsage.get(targetKey);
        if (targetSet) {
            targetSet.delete(link.id);
            if (targetSet.size === 0) {
                this.edgeUsage.delete(targetKey);
            }
        }

        // Cache invalidation not needed: assignmentKey in cache key automatically handles state changes
    }

    /**
     * Get valid domain for a link (filter by hard constraints)
     */
    getValidDomain(link) {
        const sourceNode = this.nodeMap.get(link.source);
        const targetNode = this.nodeMap.get(link.target);

        if (!sourceNode || !targetNode) return [];

        const validCombos = [];
        const edges = ['top', 'bottom', 'left', 'right'];

        for (const sourceEdge of edges) {
            for (const targetEdge of edges) {
                // Calculate combo
                const combo = {
                    sourceEdge,
                    targetEdge,
                    sourcePoint: this.optimizer.getEdgeCenterPoint(sourceNode, sourceEdge),
                    targetPoint: this.optimizer.getEdgeCenterPoint(targetNode, targetEdge)
                };

                const dx = combo.targetPoint.x - combo.sourcePoint.x;
                const dy = combo.targetPoint.y - combo.sourcePoint.y;
                combo.distance = Math.sqrt(dx * dx + dy * dy);

                // Validate hard constraints
                if (HardConstraints.validateAll(
                    link, sourceEdge, targetEdge, combo, sourceNode, targetNode,
                    this.nodes, this.assignment, this.reverseLinkMap, this.optimizer
                )) {
                    validCombos.push(combo);
                }
            }
        }

        return validCombos;
    }

    /**
     * Backtracking search with constraint propagation
     */
    backtrack(linkIndex) {
        this.nodeCount++;

        // Check cancellation (for background optimization)
        if (this.cancelled) {
            log('[CSP BACKTRACK] Cancelled by user input');
            return false; // Stop search
        }

        // Base case: All links assigned
        if (linkIndex >= this.sortedLinks.length) {
            // Calculate total score using pre-calculated scores
            // Performance: O(m) sum instead of O(m) recalculation
            let totalScore = 0;
            for (const [linkId, assignment] of this.assignment) {
                totalScore += assignment.score; // Use stored score
            }

            // Update best solution
            if (totalScore < this.bestScore) {
                const previousScore = this.bestScore;
                const improvement = previousScore === Infinity ? totalScore : previousScore - totalScore;

                this.bestScore = totalScore;
                this.bestAssignment = new Map(this.assignment);
                console.log(`[CSP SOLUTION] Found solution with score=${totalScore.toFixed(1)} (nodes=${this.nodeCount}, prunes=${this.pruneCount})`);

                // Log assignment details for debugging
                for (const [linkId, assignment] of this.assignment) {
                    const link = this.linkMap.get(linkId);
                    if (link) {
                        console.log(`  ${link.source}→${link.target}: ${assignment.sourceEdge}→${assignment.targetEdge} (score=${assignment.score.toFixed(1)})`);
                    }
                }

                // Progressive callback - solution improved
                if (this.onProgressCallback) {
                    this.onProgressCallback({
                        type: 'solution_improved',
                        assignment: new Map(this.bestAssignment),
                        score: this.bestScore,
                        previousScore: previousScore,
                        improvement: improvement,
                        progress: linkIndex / this.sortedLinks.length,
                        stats: {
                            nodeCount: this.nodeCount,
                            pruneCount: this.pruneCount,
                            elapsedMs: performance.now() - this.startTime
                        }
                    });
                }

                // Early termination: Perfect solution found (no crossings)
                if (this.bestScore === 0) {
                    console.log(`[CSP SOLUTION] Perfect solution (score=0), stopping search`);
                    return true; // Stop immediately
                }
            }

            return true; // Found a solution
        }

        const link = this.sortedLinks[linkIndex];

        // Get valid domain (filtered by hard constraints)
        const validDomain = this.getValidDomain(link);

        if (validDomain.length === 0) {
            log(`[CSP BACKTRACK] No valid domain for ${link.source}→${link.target} (prune)`);
            this.pruneCount++;
            return false; // Backtrack
        }

        log(`[CSP] ${link.source}→${link.target}: ${validDomain.length} valid combinations`);

        // Calculate current partial score once (outside combo loop)
        // Performance: O(m) once instead of O(m × domain_size)
        let currentPartialScore = 0;
        if (this.bestScore !== Infinity) {
            for (const [linkId, assignment] of this.assignment) {
                currentPartialScore += assignment.score; // Use stored score
            }
        }

        // Try each value in domain (on-demand scoring)
        for (const combo of validDomain) {
            // Calculate score on-demand (only when actually trying this combo)
            const score = SoftConstraints.calculateTotal(
                link, combo, this.assignment,
                this.reverseLinkMap, this
            );
            log(`  [CSP TRY] ${combo.sourceEdge}→${combo.targetEdge}, score=${score.toFixed(1)}`);

            // Early pruning: If partial score already exceeds best, skip
            // Performance: Reuse pre-calculated currentPartialScore (O(1) check)
            if (this.bestScore !== Infinity) {
                if (currentPartialScore + score > this.bestScore) {
                    log(`  [CSP PRUNE] Partial score ${(currentPartialScore + score).toFixed(1)} > best ${this.bestScore.toFixed(1)}`);
                    this.pruneCount++;
                    continue; // Skip this branch
                }
            }

            // Assign with edge usage tracking
            this.assignEdgeRouting(link, combo.sourceEdge, combo.targetEdge, combo);

            // Recursive backtrack
            const success = this.backtrack(linkIndex + 1);

            if (success) {
                // Early termination: Stop if perfect solution found
                if (this.bestScore === 0) {
                    return true; // Propagate early termination up the call stack
                }
                // Otherwise continue searching for better solutions
            }

            // Unassign and update edge usage tracking (backtrack)
            this.unassignEdgeRouting(link);
        }

        return true; // Explored all branches
    }

    /**
     * Solve the CSP
     */
    solve() {
        // Clear snap point cache for fresh start
        SoftConstraints.clearCache();

        log('[CSP SOLVER] Starting constraint satisfaction solver...');
        log(`[CSP] Variables: ${this.sortedLinks.length} links`);
        log(`[CSP] Domain size per variable: up to 16 combinations`);
        log(`[CSP] Hard constraints: 4 (initial blocking, node collisions, min distance, bidirectional)`);
        log(`[CSP] Soft constraints: 3 (intersections weight=50000, distance weight=1, symmetry bonus=-5000)`);
        log(`[CSP] Variable ordering: MRV (Minimum Remaining Values) heuristic`);

        const startTime = performance.now();
        this.startTime = startTime; // Store for progress reporting

        this.backtrack(0);

        const endTime = performance.now();
        const elapsedMs = endTime - startTime;

        // Check if cancelled
        if (this.cancelled) {
            console.log(`[CSP SOLVER] Cancelled after ${elapsedMs.toFixed(1)}ms (nodes=${this.nodeCount})`);
            return null; // Return null to indicate cancellation
        }

        if (this.bestAssignment) {
            // Always log final results (not debug-gated)
            // console.log(`[CSP SOLVER] Solution found with score: ${this.bestScore.toFixed(1)}`);
            // console.log(`[CSP SOLVER] Search statistics: nodes=${this.nodeCount}, prunes=${this.pruneCount}, time=${elapsedMs.toFixed(1)}ms`);
            // console.log(`[CSP SOLVER] Cache statistics: size=${SoftConstraints.snapPointCache.size} entries`);
            return this.bestAssignment;
        } else {
            // console.warn('[CSP SOLVER] No solution found!');
            return null;
        }
    }
}
