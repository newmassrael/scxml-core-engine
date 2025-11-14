// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * TransitionLayoutOptimizer
 *
 * Optimizes transition arrow layout for minimal crossing and optimal snap positioning.
 * Handles connection analysis, snap point calculation, and crossing minimization.
 */

// Debug flag - set to true to enable detailed logging
const DEBUG_LAYOUT_OPTIMIZER = false;

// No-op logging function for production (reuse if already defined by constraint-solver.js)
// Use global assignment to avoid const/var declaration conflicts
if (typeof log === 'undefined') {
    log = DEBUG_LAYOUT_OPTIMIZER ? console.log.bind(console) : () => {};
}

class TransitionLayoutOptimizer {
    // Node size constants (half-width and half-height)
    static NODE_SIZES = {
        'initial-pseudo': { halfWidth: 10, halfHeight: 10 },
        'atomic': { halfWidth: 30, halfHeight: 20 },
        'compound': { halfWidth: 30, halfHeight: 20 },
        'final': { halfWidth: 30, halfHeight: 20 },
        'history': { halfWidth: 15, halfHeight: 15 },
        'parallel': { halfWidth: 30, halfHeight: 20 }
    };

    // Path segment constants
    static MIN_SEGMENT_LENGTH = 30;
    static MIN_SAFE_DISTANCE = 60; // MIN_SEGMENT_LENGTH * 2

    // Progressive optimization configuration
    static PROGRESS_RENDER_INTERVAL_MS = 50;  // Minimum interval between progressive renders
    static CSP_THRESHOLD = 15;  // Maximum links for CSP optimization

    /**
     * Get node size by type or from node object
     * @param {string|Object} nodeOrType - Node object or node type string
     * @returns {Object} {halfWidth, halfHeight}
     */
    static getNodeSize(nodeOrType) {
        // If it's a node object with width/height, use actual dimensions
        if (typeof nodeOrType === 'object' && nodeOrType !== null) {
            const node = nodeOrType;

            // Check if node is collapsed - use minimum size for collapsed compound/parallel states
            if (node.collapsed && (node.type === 'compound' || node.type === 'parallel')) {
                // Use LAYOUT_CONSTANTS from visualizer-core.js
                if (typeof LAYOUT_CONSTANTS !== 'undefined') {
                    return {
                        halfWidth: LAYOUT_CONSTANTS.STATE_MIN_WIDTH / 2,
                        halfHeight: LAYOUT_CONSTANTS.STATE_MIN_HEIGHT / 2
                    };
                }
                // Fallback if LAYOUT_CONSTANTS not available
                return {
                    halfWidth: 70,   // Default STATE_MIN_WIDTH/2
                    halfHeight: 25   // Default STATE_MIN_HEIGHT/2
                };
            }

            if (node.width !== undefined && node.height !== undefined) {
                return {
                    halfWidth: node.width / 2,
                    halfHeight: node.height / 2
                };
            }
            // Fallback to type-based lookup if no dimensions
            nodeOrType = node.type;
        }
        // Type-based lookup for nodes without dimensions
        return this.NODE_SIZES[nodeOrType] || { halfWidth: 30, halfHeight: 20 };
    }

    constructor(nodes, links) {
        this.nodes = nodes;
        this.links = links;
        this.cspRunning = false; // Flag to prevent concurrent CSP executions

        // Initialize helper modules
        this.snapCalculator = new SnapCalculator(this);
        this.cspSolver = new CSPSolver(this);
        this.pathUtils = new PathUtils(this);
    }

    _sortLinksByPriority(links) {
        return [...links].sort((a, b) => {
            if (a.linkType === 'initial' && b.linkType !== 'initial') return -1;
            if (a.linkType !== 'initial' && b.linkType === 'initial') return 1;
            return 0;
        });
    }

    _applySolutionToLinks(assignment, links, nodes) {
        // Note: Expects already filtered links (transition and initial only)

        // Convert different formats to uniform format
        let assignments;

        if (assignment instanceof Map) {
            // Progress case: Map → Array of tuples [[linkId, {sourceEdge, targetEdge}], ...]
            assignments = Array.from(assignment.entries()).map(([linkId, value]) => ({
                linkId,
                sourceEdge: value.sourceEdge,
                targetEdge: value.targetEdge
            }));
        } else if (Array.isArray(assignment)) {
            // Solution case: Already array of objects [{linkId, sourceEdge, targetEdge}, ...]
            assignments = assignment;
        } else {
            console.error('[OPTIMIZE] Invalid assignment format:', assignment);
            return;
        }

        // Apply routing to links
        assignments.forEach(({ linkId, sourceEdge, targetEdge }) => {
            const link = links.find(l => l.id === linkId);
            if (link) {
                link.routing = RoutingState.fromEdges(sourceEdge, targetEdge);
            }
        });

        // Sort and redistribute snap points
        const sortedLinks = this._sortLinksByPriority(links);
        this.distributeSnapPointsOnEdges(sortedLinks, nodes);
    }

    // Delegate methods to helper modules
    predictEdge(source, target, isSourceEdge) { return this.snapCalculator.predictEdge(source, target, isSourceEdge); }
    hasInitialTransitionOnEdge(nodeId, edge) { return this.snapCalculator.hasInitialTransitionOnEdge(nodeId, edge); }
    countConnectionsOnEdge(nodeId, edge) { return this.snapCalculator.countConnectionsOnEdge(nodeId, edge); }
    estimateSourceSnapPosition(sourceId, targetId, link) { return this.snapCalculator.estimateSourceSnapPosition(sourceId, targetId, link); }
    calculateSnapPosition(nodeId, edge, linkId, direction) { return this.snapCalculator.calculateSnapPosition(nodeId, edge, linkId, direction); }
    getAllPossibleSnapCombinations(link, sourceNode, targetNode, reverseRouting) { return this.snapCalculator.getAllPossibleSnapCombinations(link, sourceNode, targetNode, reverseRouting); }
    getEdgeCenterPoint(node, edge) { return this.snapCalculator.getEdgeCenterPoint(node, edge); }
    calculateComboScore(link, combo, existingAssignment, allLinks) { return this.snapCalculator.calculateComboScore(link, combo, existingAssignment, allLinks); }
    distributeSnapPointsOnEdges(links, nodes) { return this.snapCalculator.distributeSnapPointsOnEdges(links, nodes); }

    calculatePathIntersections(path1, path2) { return this.pathUtils.calculatePathIntersections(path1, path2); }
    getPathSegments(path) { return this.pathUtils.getPathSegments(path); }
    segmentsIntersect(seg1, seg2) { return this.pathUtils.segmentsIntersect(seg1, seg2); }
    pathIntersectsNode(path, node, options) { return this.pathUtils.pathIntersectsNode(path, node, options); }
    segmentIntersectsRect(segment, left, top, right, bottom) { return this.pathUtils.segmentIntersectsRect(segment, left, top, right, bottom); }
    segmentIntersectsRectExcludingEndpoint(segment, left, top, right, bottom, endpoint) { return this.pathUtils.segmentIntersectsRectExcludingEndpoint(segment, left, top, right, bottom, endpoint); }
    evaluateCombination(link, sourceNode, targetNode, sourceEdge, targetEdge, assignedPaths, nodes) { return this.pathUtils.evaluateCombination(link, sourceNode, targetNode, sourceEdge, targetEdge, assignedPaths, nodes); }

    optimizeSnapPointAssignmentsProgressive(links, nodes, draggedNodeId, onComplete, onProgress, debounceMs) { return this.cspSolver.optimizeSnapPointAssignmentsProgressive(links, nodes, draggedNodeId, onComplete, onProgress, debounceMs); }
    fallbackToMainThreadCSP(links, nodes, draggedNodeId, onComplete) { return this.cspSolver.fallbackToMainThreadCSP(links, nodes, draggedNodeId, onComplete); }
    optimizeSnapPointAssignments(links, nodes, useGreedy, draggedNodeId) { return this.cspSolver.optimizeSnapPointAssignments(links, nodes, useGreedy, draggedNodeId); }
    convertGreedyToCSPSolution(links) { return this.cspSolver.convertGreedyToCSPSolution(links); }
    runBackgroundCSPOptimization(transitionLinks, nodes, draggedNodeId) { return this.cspSolver.runBackgroundCSPOptimization(transitionLinks, nodes, draggedNodeId); }
    optimizeSnapPointAssignmentsGreedy(links, nodes, draggedNodeId) { return this.cspSolver.optimizeSnapPointAssignmentsGreedy(links, nodes, draggedNodeId); }
}
