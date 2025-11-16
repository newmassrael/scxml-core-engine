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
 *    greedy/CSP algorithm mismatch), this solver simulates the actual snap point positions
 *    that will be distributed during snap point distribution when multiple transitions share an edge.
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
    static validateInitialBlocking(link, sourceEdge, targetEdge, optimizer, visualSourceId = null, visualTargetId = null) {
        if (link.linkType === 'initial') return true;

        const sourceId = visualSourceId || link.source;
        const targetId = visualTargetId || link.target;

        if (optimizer.hasInitialTransitionOnEdge(sourceId, sourceEdge)) {
            return false; // FAIL: Source edge blocked
        }
        if (optimizer.hasInitialTransitionOnEdge(targetId, targetEdge)) {
            return false; // FAIL: Target edge blocked
        }
        return true; // PASS
    }

    /**
     * HC2: No node collisions
     * Path가 source/target 이외의 노드를 관통하지 않음
     * **Exception: Compound hierarchy (parent-child) paths are allowed to intersect parent nodes**
     */
    static validateNodeCollisions(combo, sourceNode, targetNode, allNodes, optimizer, parentChildMap = new Map()) {
        // Source node collision (skip first segment)
        if (optimizer.pathIntersectsNode(combo, sourceNode, { skipFirstSegment: true })) {
            return false; // FAIL
        }

        // Target node collision (skip last segment)
        const targetCollision = optimizer.pathIntersectsNode(combo, targetNode, { skipLastSegment: true });
        if (targetCollision) {
            log(`[HC2-TARGET] ${combo.sourceEdge}→${combo.targetEdge}: Path penetrates target node ${targetNode.id}`);
            return false; // FAIL
        }

        // Helper: Get all ancestors of a node
        const getAncestors = (nodeId) => {
            const ancestors = new Set();
            let current = nodeId;
            while (parentChildMap.has(current)) {
                const parent = parentChildMap.get(current);
                ancestors.add(parent);
                current = parent;
            }
            return ancestors;
        };

        // Helper: Get siblings of a node (nodes with same parent)
        const getSiblings = (nodeId) => {
            const siblings = new Set();
            if (parentChildMap.has(nodeId)) {
                const parent = parentChildMap.get(nodeId);
                // Find all nodes with same parent
                for (const [child, childParent] of parentChildMap.entries()) {
                    if (childParent === parent && child !== nodeId) {
                        siblings.add(child);
                    }
                }
            }
            return siblings;
        };

        // Get ancestors for source and target
        const sourceAncestors = getAncestors(sourceNode.id);
        const targetAncestors = getAncestors(targetNode.id);
        
        // Get siblings for target (for parent→child transitions)
        const targetSiblings = getSiblings(targetNode.id);

        // Other nodes collision
        for (const node of allNodes) {
            if (node.id === sourceNode.id || node.id === targetNode.id) continue;
            
            // **CRITICAL: Skip parent/ancestor nodes for compound hierarchy**
            // Visualizer layout: Parent→child transitions (e.g., p→ps1) should not be blocked by parent node collision
            if (sourceAncestors.has(node.id) || targetAncestors.has(node.id)) {
                continue; // Allow path to intersect parent/ancestor nodes
            }
            
            // **CRITICAL: Skip sibling compound nodes for parent→child transitions**
            // Visualizer layout: Parent→child transitions (e.g., p→ps1) can intersect compound siblings (user can route around)
            // BUT: Block intersection with atomic siblings (leaf nodes like ps2) - no way to route around them
            const isParentToChild = targetAncestors.has(sourceNode.id);
            if (isParentToChild && targetSiblings.has(node.id)) {
                // Allow path to intersect sibling COMPOUND states (user can route around them)
                // Block path through sibling ATOMIC states (leaf nodes - cannot route around)
                if (node.type !== 'atomic') {
                    log(`[HC2-SIBLING] Allowing compound sibling ${node.id} (type=${node.type})`);
                    continue; // Allow intersection with compound siblings
                }
                // Fall through: Check atomic sibling collision (will FAIL if intersects)
                log(`[HC2-SIBLING] Checking atomic sibling ${node.id} collision...`);
            }
            
            // Check actual intersection
            const intersects = optimizer.pathIntersectsNode(combo, node);
            
            if (intersects) {
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
    static validateBidirectionalConflict(link, sourceEdge, targetEdge, assignment, reverseLinkMap, visualSourceId = null, visualTargetId = null) {
        const sourceId = visualSourceId || link.source;
        const targetId = visualTargetId || link.target;
        const reverseKey = `${targetId}→${sourceId}`;
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
                       allNodes, assignment, reverseLinkMap, optimizer, debugLinkIndex = -1, parentChildMap = new Map()) {
        const debug = debugLinkIndex >= 0 && debugLinkIndex < 3; // Debug first 3 links
        
        if (!this.validateInitialBlocking(link, sourceEdge, targetEdge, optimizer, sourceNode.id, targetNode.id)) {
            if (debug) console.log(`  [HC FAIL] ${sourceNode.id}→${targetNode.id} ${sourceEdge}→${targetEdge}: Initial blocking`);
            return false;
        }
        if (!this.validateNodeCollisions(combo, sourceNode, targetNode, allNodes, optimizer, parentChildMap)) {
            if (debug) console.log(`  [HC FAIL] ${link.source}→${link.target} ${sourceEdge}→${targetEdge}: Node collision`);
            return false;
        }
        if (!this.validateMinimumDistance(combo)) {
            if (debug) console.log(`  [HC FAIL] ${link.source}→${link.target} ${sourceEdge}→${targetEdge}: Minimum distance`);
            return false;
        }
        if (!this.validateBidirectionalConflict(link, sourceEdge, targetEdge, assignment, reverseLinkMap, sourceNode.id, targetNode.id)) {
            if (debug) console.log(`  [HC FAIL] ${sourceNode.id}→${targetNode.id} ${sourceEdge}→${targetEdge}: Bidirectional conflict`);
            return false;
        }
        return true;
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
     * Matches snap point distribution logic in TransitionLayoutOptimizer.distributeSnapPointsOnEdges()
     */
    static simulateSnapPoints(link, combo, assignment, solver) {
        const sourceNode = solver.nodeMap.get(solver.getVisualSource(link));
        const targetNode = solver.nodeMap.get(solver.getVisualTarget(link));

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
            const aOther = solver.nodeMap.get(a.isSource ? solver.getVisualTarget(a.link) : solver.getVisualSource(a.link));
            const bOther = solver.nodeMap.get(b.isSource ? solver.getVisualTarget(b.link) : solver.getVisualSource(b.link));

            if (!aOther || !bOther) return 0;

            if (edge === 'top' || edge === 'bottom') {
                return (aOther.x || 0) - (bOther.x || 0);
            } else {
                return (aOther.y || 0) - (bOther.y || 0);
            }
        };

        incoming.sort(sortByOtherNode);
        outgoing.sort(sortByOtherNode);

        // Combine: incoming first, then outgoing (matches snap point distribution)
        const sortedLinks = [...incoming, ...outgoing];
        const totalCount = sortedLinks.length;

        // Find index of current link
        const currentIndex = sortedLinks.findIndex(item => item.isCurrentLink);
        if (currentIndex < 0) {
            console.error('[SIMULATE ERROR] Current link not found in sorted list');
            return optimizer.getEdgeCenterPoint(node, edge);
        }

        // Calculate position (matches snap point distribution)
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
    static scoreSymmetry(link, sourceEdge, targetEdge, assignment, reverseLinkMap, visualSourceId = null, visualTargetId = null) {
        const sourceId = visualSourceId || link.source;
        const targetId = visualTargetId || link.target;
        const reverseKey = `${targetId}→${sourceId}`;
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
     * SC5: Penalize sibling node overlap for parent→child transitions
     * Weight: 50000 (critical - same as SC1 intersection)
     * 
     * Context: HC2 allows parent→child transitions to penetrate sibling nodes for visualizer layout requirements,
     * but we want CSP to prefer non-overlapping paths when possible.
     * 
     * Example: p→ps1 transition may penetrate ps2 (sibling), but should avoid if alternative exists.
     */
    static scoreSiblingOverlap(link, combo, solver) {
        const sourceNode = solver.nodeMap.get(solver.getVisualSource(link));
        const targetNode = solver.nodeMap.get(solver.getVisualTarget(link));

        if (!sourceNode || !targetNode) {
            return 0;
        }

        // Check if this is a parent→child transition
        const targetAncestors = new Set();
        let current = targetNode.id;
        while (solver.parentChildMap.has(current)) {
            const parent = solver.parentChildMap.get(current);
            targetAncestors.add(parent);
            current = parent;
        }

        const isParentToChild = targetAncestors.has(sourceNode.id);

        if (!isParentToChild) {
            return 0; // Not a parent→child transition, no sibling overlap concern
        }

        // Get target's siblings
        const targetSiblings = new Set();
        if (solver.parentChildMap.has(targetNode.id)) {
            const parent = solver.parentChildMap.get(targetNode.id);
            for (const [child, childParent] of solver.parentChildMap.entries()) {
                if (childParent === parent && child !== targetNode.id) {
                    targetSiblings.add(child);
                }
            }
        }

        if (targetSiblings.size === 0) {
            return 0; // No siblings to check
        }

        // Check if path overlaps any sibling nodes
        let overlapCount = 0;
        for (const siblingId of targetSiblings) {
            const siblingNode = solver.nodeMap.get(siblingId);
            if (!siblingNode) {
                continue;
            }

            const overlaps = solver.optimizer.pathIntersectsNode(combo, siblingNode);

            if (overlaps) {
                overlapCount++;
            }
        }

        if (overlapCount > 0) {
            const penalty = overlapCount * 50000;
            return penalty;
        }

        return 0;
    }

    /**
     * Calculate total soft constraint score
     */
    static calculateTotal(link, combo, assignment, reverseLinkMap, solver) {
        const intersectionScore = this.scoreIntersections(link, combo, assignment, solver);
        const distanceScore = this.scoreDistance(combo);
        const symmetryBonus = this.scoreSymmetry(link, combo.sourceEdge, combo.targetEdge,
                                                 assignment, reverseLinkMap, solver.getVisualSource(link), solver.getVisualTarget(link));
        const selfOverlapScore = this.scoreSelfOverlap(combo);
        const siblingOverlapScore = this.scoreSiblingOverlap(link, combo, solver);

        return intersectionScore + distanceScore + symmetryBonus + selfOverlapScore + siblingOverlapScore;
    }
}

/**
 * ConstraintSolver - Backtracking search with constraint propagation
 */
class ConstraintSolver {
    // Helper methods for visual redirect support
    getVisualSource(link) {
        return link.visualSource || link.source;
    }

    getVisualTarget(link) {
        return link.visualTarget || link.target;
    }

    constructor(links, nodes, optimizer, parentChildMap = new Map(), greedySolution = null, draggedNodeId = null) {
        this.links = links;
        this.nodes = nodes;
        this.optimizer = optimizer;
        this.parentChildMap = parentChildMap; // Map<childId, parentId>

        // Performance Optimization: O(1) lookup maps
        this.nodeMap = new Map(nodes.map(n => [n.id, n]));
        this.linkMap = new Map(links.map(l => [l.id, l]));

        // Build reverse link map (using visual node IDs for collapsed state support)
        this.reverseLinkMap = new Map();
        links.forEach(link => {
            const visualSource = link.visualSource || link.source;
            const visualTarget = link.visualTarget || link.target;
            const key = `${visualSource}→${visualTarget}`;
            this.reverseLinkMap.set(key, link);
        });

        // Calculate distance from dragged node for locality-aware optimization
        const getDraggedNodeDistance = (link) => {
            if (!draggedNodeId) return 0;

            const sourceNode = this.nodeMap.get(this.getVisualSource(link));
            const targetNode = this.nodeMap.get(this.getVisualTarget(link));
            const draggedNode = this.nodeMap.get(draggedNodeId);

            if (!sourceNode || !targetNode || !draggedNode) return Infinity;

            // Calculate minimum distance from link endpoints to dragged node
            const sourceDist = Math.hypot(sourceNode.x - draggedNode.x, sourceNode.y - draggedNode.y);
            const targetDist = Math.hypot(targetNode.x - draggedNode.x, targetNode.y - draggedNode.y);
            return Math.min(sourceDist, targetDist);
        };

        // Priority: Most constrained first + Locality-aware
        // 1. Initial transitions (highest priority - must not be blocked)
        // 2. Parent→child transitions (e.g., p→ps1 - constrained by parent edge availability)
        // 3. Links close to dragged node (locality)
        // 4. Other transitions
        this.sortedLinks = [...links].sort((a, b) => {
            const aIsInitial = a.linkType === 'initial';
            const bIsInitial = b.linkType === 'initial';
            const aIsParentToChild = parentChildMap.has(a.target) && parentChildMap.get(a.target) === a.source;
            const bIsParentToChild = parentChildMap.has(b.target) && parentChildMap.get(b.target) === b.source;

            // Initial transitions first
            if (aIsInitial && !bIsInitial) return -1;
            if (!aIsInitial && bIsInitial) return 1;

            // Then parent→child transitions
            if (aIsParentToChild && !bIsParentToChild) return -1;
            if (!aIsParentToChild && bIsParentToChild) return 1;

            // Then by distance to dragged node (closest first)
            if (draggedNodeId) {
                const aDist = getDraggedNodeDistance(a);
                const bDist = getDraggedNodeDistance(b);
                return aDist - bDist;
            }

            return 0; // Other transitions (preserve original order)
        });

        // Log sorted link order with dragged node context
        if (draggedNodeId) {
            console.log(`[CSP] Variable ordering (locality-aware, dragged node: ${draggedNodeId}):`);
        } else {
            console.log('[CSP] Variable ordering (links to assign):');
        }
        this.sortedLinks.forEach((link, index) => {
            const type = link.linkType === 'initial' ? '(initial)' :
                        (parentChildMap.has(link.target) && parentChildMap.get(link.target) === link.source) ? '(parent→child)' :
                        '';
            const dist = draggedNodeId ? getDraggedNodeDistance(link) : null;
            const distTag = dist !== null && dist !== Infinity ? ` [dist=${dist.toFixed(0)}px]` : '';
            console.log(`  ${index}. ${link.source}→${link.target} ${type}${distTag}`);
        });

        // Assignment: Map<linkId, {sourceEdge, targetEdge, combo}>
        this.assignment = new Map();

        // Performance Optimization: Edge usage tracking
        // Map<"nodeId:edge", Set<linkId>>
        this.edgeUsage = new Map();

        // Best solution tracking
        this.bestAssignment = null;
        this.bestScore = Infinity;

        // Greedy solution warm-start (Strategy 1 + 3: Initial solution + Value ordering)
        this.greedyPreferences = new Map(); // Map<linkId, {sourceEdge, targetEdge}>
        if (greedySolution) {
            console.log('[CSP WARM-START] Initializing with greedy solution...');
            this.bestAssignment = greedySolution.assignment;
            this.bestScore = greedySolution.score;
            this.greedyPreferences = greedySolution.preferences;
            console.log(`[CSP WARM-START] Baseline score=${this.bestScore.toFixed(1)} from greedy algorithm`);
            console.log(`[CSP WARM-START] Greedy preferences for ${this.greedyPreferences.size} links`);
        }

        // Statistics
        this.nodeCount = 0;
        this.pruneCount = 0;

        // Cancellation support for background optimization
        this.cancelled = false;

        // Progress callback for incremental updates
        this.onProgressCallback = null;
        this.startTime = 0;

        // Early termination limits (prevent infinite search)
        // Reduced for background optimization to avoid blocking UI
        // 250ms per iteration, 8 iterations = 2000ms total progressive refinement
        this.TIME_LIMIT_MS = 250; // 250ms per iteration for progressive improvement
        this.NO_IMPROVEMENT_LIMIT = 10000; // Stop after 10k nodes without improvement
        this.lastImprovementNode = 0;
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
        const sourceKey = `${this.getVisualSource(link)}:${sourceEdge}`;
        if (!this.edgeUsage.has(sourceKey)) {
            this.edgeUsage.set(sourceKey, new Set());
        }
        this.edgeUsage.get(sourceKey).add(link.id);

        // Track target edge usage
        const targetKey = `${this.getVisualTarget(link)}:${targetEdge}`;
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
    getValidDomain(link, linkIndex = -1) {
        const sourceNode = this.nodeMap.get(this.getVisualSource(link));
        const targetNode = this.nodeMap.get(this.getVisualTarget(link));

        if (!sourceNode || !targetNode) return [];

        // Debug p→ps1 link
        const debugLink = link.source === 'p' && link.target === 'ps1';
        if (debugLink) {
            console.log(`[DEBUG] Validating p→ps1 link (linkIndex=${linkIndex})`);
            console.log(`  Source node p:`, sourceNode);
            console.log(`  Target node ps1:`, targetNode);
            console.log(`  Distance: ${Math.sqrt(Math.pow(targetNode.x - sourceNode.x, 2) + Math.pow(targetNode.y - sourceNode.y, 2)).toFixed(1)}px`);
            console.log(`  Parent-child map:`, Array.from(this.parentChildMap.entries()));
            console.log(`  Current assignment:`, Array.from(this.assignment.entries()).map(([id, a]) => {
                const l = this.linkMap.get(id);
                return `${l.source}→${l.target}: ${a.sourceEdge}→${a.targetEdge}`;
            }));
        }

        const validCombos = [];
        const edges = ['top', 'bottom', 'left', 'right'];
        const rejectionReasons = new Map(); // Track why each combo fails

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

                // Validate each hard constraint individually for debugging
                let passed = true;
                let reason = '';

                if (!HardConstraints.validateInitialBlocking(link, sourceEdge, targetEdge, this.optimizer, this.getVisualSource(link), this.getVisualTarget(link))) {
                    passed = false;
                    reason = 'HC1: Initial blocking';
                } else if (!HardConstraints.validateNodeCollisions(combo, sourceNode, targetNode, this.nodes, this.optimizer, this.parentChildMap)) {
                    passed = false;
                    reason = 'HC2: Node collision';
                } else if (!HardConstraints.validateMinimumDistance(combo)) {
                    passed = false;
                    reason = 'HC3: Minimum distance';
                } else if (!HardConstraints.validateBidirectionalConflict(link, sourceEdge, targetEdge, this.assignment, this.reverseLinkMap, this.getVisualSource(link), this.getVisualTarget(link))) {
                    passed = false;
                    reason = 'HC4: Bidirectional conflict';
                }

                if (passed) {
                    validCombos.push(combo);
                } else if (debugLink) {
                    rejectionReasons.set(`${sourceEdge}→${targetEdge}`, reason);
                }
            }
        }

        if (debugLink) {
            console.log(`[DEBUG] p→ps1 validation results: ${validCombos.length}/16 combos passed`);
            console.log(`  Rejection reasons:`);
            for (const [combo, reason] of rejectionReasons.entries()) {
                console.log(`    ${combo}: ${reason}`);
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

        // Check time limit (prevent infinite search)
        if (performance.now() - this.startTime > this.TIME_LIMIT_MS) {
            console.log(`[CSP BACKTRACK] Time limit reached (${this.TIME_LIMIT_MS}ms), stopping with best solution found`);
            return false; // Stop search
        }

        // Check no-improvement limit (prevent infinite search)
        if (this.nodeCount - this.lastImprovementNode > this.NO_IMPROVEMENT_LIMIT) {
            console.log(`[CSP BACKTRACK] No improvement for ${this.NO_IMPROVEMENT_LIMIT} nodes, stopping with best solution found`);
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
                this.lastImprovementNode = this.nodeCount; // Reset no-improvement counter
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
        const validDomain = this.getValidDomain(link, linkIndex);

        if (validDomain.length === 0) {
            log(`[CSP BACKTRACK] No valid domain for ${link.source}→${link.target} (prune)`);
            // Log early link failures (helps debug complete failures)
            if (linkIndex < 3) {
                const visualSource = this.getVisualSource(link);
                const visualTarget = this.getVisualTarget(link);
                const redirectLabel = (visualSource !== link.source || visualTarget !== link.target)
                    ? ` (visual: ${visualSource}→${visualTarget})` : '';
                console.log(`[CSP] Link #${linkIndex} ${link.source}→${link.target}${redirectLabel} has no valid domain - all combinations rejected by hard constraints`);
            }
            this.pruneCount++;
            return false; // Backtrack
        }

        // Log domain size for early links (helps debug)
        if (linkIndex < 3) {
            const visualSource = this.getVisualSource(link);
            const visualTarget = this.getVisualTarget(link);
            const redirectLabel = (visualSource !== link.source || visualTarget !== link.target)
                ? ` (visual: ${visualSource}→${visualTarget})` : '';
            console.log(`[CSP] Link #${linkIndex} ${link.source}→${link.target}${redirectLabel}: ${validDomain.length} valid combinations`);
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

        // Value Ordering Heuristic: Score all combos and sort by score (best first)
        // Strategy 3: Prioritize greedy preferences + score-based ordering
        const scoredDomain = validDomain.map(combo => {
            const score = SoftConstraints.calculateTotal(
                link, combo, this.assignment,
                this.reverseLinkMap, this
            );

            // Check if this combo matches greedy preference
            const greedyPref = this.greedyPreferences.get(link.id);
            const isGreedyPreferred = greedyPref &&
                combo.sourceEdge === greedyPref.sourceEdge &&
                combo.targetEdge === greedyPref.targetEdge;

            return { combo, score, isGreedyPreferred };
        });

        // Sort by greedy preference first, then by score (ascending = best first)
        scoredDomain.sort((a, b) => {
            // Prioritize greedy preferences
            if (a.isGreedyPreferred && !b.isGreedyPreferred) return -1;
            if (!a.isGreedyPreferred && b.isGreedyPreferred) return 1;

            // Then sort by score
            return a.score - b.score;
        });

        // Always log value ordering for first few links (debugging warm-start)
        if (scoredDomain.length > 0 && linkIndex < 3) {
            const greedyTag = scoredDomain[0].isGreedyPreferred ? ' (greedy)' : '';
            console.log(`[CSP VALUE ORDER] Link #${linkIndex}: Sorted ${scoredDomain.length} combos, best score=${scoredDomain[0].score.toFixed(1)}${greedyTag}`);
        }

        // Try each value in domain (best-first order)
        for (const { combo, score } of scoredDomain) {
            // Early pruning: If partial score already exceeds best, skip
            if (this.bestScore !== Infinity) {
                if (currentPartialScore + score > this.bestScore) {
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

        console.log('[CSP SOLVER] Starting constraint satisfaction solver...');
        console.log(`[CSP] Variables: ${this.sortedLinks.length} links`);
        console.log(`[CSP] Domain size per variable: up to 16 combinations`);
        console.log(`[CSP] Hard constraints: 4 (initial blocking, node collisions, min distance, bidirectional)`);
        console.log(`[CSP] Soft constraints: 5 (SC1: intersections=50000, SC2: distance=1, SC3: symmetry=-5000, SC4: self-overlap=50000, SC5: sibling-overlap=50000)`);
        console.log(`[CSP] Variable ordering: MRV (Minimum Remaining Values) heuristic`);

        const startTime = performance.now();
        this.startTime = startTime; // Store for progress reporting
        
        console.log('[CSP BACKTRACK] Starting backtracking search with value ordering...');

        this.backtrack(0);

        const endTime = performance.now();
        const elapsedMs = endTime - startTime;

        // Check if cancelled
        if (this.cancelled) {
            console.log(`[CSP SOLVER] Cancelled after ${elapsedMs.toFixed(1)}ms (nodes=${this.nodeCount})`);
            return null; // Return null to indicate cancellation
        }

        if (this.bestAssignment) {
            // Log final optimization results
            console.log(`[CSP SOLVER] Solution found with score=${this.bestScore.toFixed(1)}`);
            console.log(`[CSP SOLVER] Statistics: nodes=${this.nodeCount}, prunes=${this.pruneCount}, time=${elapsedMs.toFixed(1)}ms`);
            const pruneRate = ((this.pruneCount / Math.max(this.nodeCount, 1)) * 100).toFixed(1);
            console.log(`[CSP SOLVER] Prune efficiency: ${pruneRate}% (${this.pruneCount} prunes / ${this.nodeCount} nodes)`);
            return this.bestAssignment;
        } else {
            console.log('[CSP SOLVER] No solution found - using greedy result');
            console.log(`[CSP SOLVER] Statistics: nodes=${this.nodeCount}, prunes=${this.pruneCount}, time=${elapsedMs.toFixed(1)}ms`);
            console.log(`[CSP SOLVER] Explored ${this.nodeCount} nodes, ${this.pruneCount} prunes (${((this.pruneCount/this.nodeCount)*100).toFixed(1)}% prune rate)`);
            if (this.nodeCount < 100) {
                console.log(`[CSP SOLVER] Very few nodes explored - likely early constraint failure`);
            }
            return null;
        }
    }
}
