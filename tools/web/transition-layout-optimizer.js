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

    /**
     * @param {Array} nodes - Array of state nodes
     * @param {Array} links - Array of transition links
     */
    constructor(nodes, links) {
        this.nodes = nodes;
        this.links = links;
        this.cspRunning = false; // Flag to prevent concurrent CSP executions
    }

    /**
     * Predict which edge a link uses based on source/target positions
     * @param {Object} source - Source node
     * @param {Object} target - Target node
     * @param {boolean} isSourceEdge - True for source edge, false for target edge
     * @returns {string} Edge name: 'top', 'bottom', 'left', or 'right'
     */
    predictEdge(source, target, isSourceEdge) {
        const sx = source.x || 0;
        const sy = source.y || 0;
        const tx = target.x || 0;
        const ty = target.y || 0;

        const dx = Math.abs(tx - sx);
        const dy = Math.abs(ty - sy);

        if (isSourceEdge) {
            // Predict which edge the path exits from
            if (dy < 30) {
                // Near-horizontal alignment: use left/right
                return tx > sx ? 'right' : 'left';
            } else if (dx < 30) {
                // Near-vertical alignment: use top/bottom
                return sy < ty ? 'bottom' : 'top';
            } else {
                // **FIXED: Choose direction based on larger distance**
                // If horizontal distance is greater, use horizontal edge
                // If vertical distance is greater, use vertical edge (Z-path)
                if (dx > dy) {
                    return tx > sx ? 'right' : 'left';
                } else {
                    return sy < ty ? 'bottom' : 'top';
                }
            }
        } else {
            // Predict which edge the path enters from
            if (dy < 30) {
                // Near-horizontal alignment: use left/right
                return tx > sx ? 'left' : 'right';
            } else if (dx < 30) {
                // Near-vertical alignment: use top/bottom
                return sy > ty ? 'bottom' : 'top';
            } else {
                // **FIXED: Choose direction based on larger distance**
                // If horizontal distance is greater, use horizontal edge
                // If vertical distance is greater, use vertical edge (Z-path)
                if (dx > dy) {
                    return tx > sx ? 'left' : 'right';
                } else {
                    // sy > ty: source below target → path goes UP → enters target's BOTTOM
                    // sy < ty: source above target → path goes DOWN → enters target's TOP
                    return sy > ty ? 'bottom' : 'top';
                }
            }
        }
    }

    /**
     * Check if a specific edge has an initial transition
     * @param {string} nodeId - Node ID
     * @param {string} edge - Edge name
     * @returns {boolean} True if initial transition exists on this edge
     */
    hasInitialTransitionOnEdge(nodeId, edge) {
        return this.links.some(link => {
            if (link.linkType !== 'initial') return false;
            if (link.target !== nodeId) return false;

            // **PRIORITY: Use actual routing if available, fallback to prediction**
            if (link.routing && link.routing.targetEdge) {
                return link.routing.targetEdge === edge;
            }

            // Fallback: Predict edge based on node positions
            const source = this.nodes.find(n => n.id === link.source);
            const target = this.nodes.find(n => n.id === link.target);
            if (!source || !target) return false;

            const targetEdge = this.predictEdge(source, target, false);
            return targetEdge === edge;
        });
    }

    /**
     * Count all connections (incoming + outgoing) on a specific edge of a node
     * @param {string} nodeId - Node ID
     * @param {string} edge - Edge name ('top', 'bottom', 'left', 'right')
     * @returns {Array} Array of {link, isSource, sourceNode, targetNode, otherNode}
     */
    countConnectionsOnEdge(nodeId, edge) {
        const node = this.nodes.find(n => n.id === nodeId);
        if (!node) return [];

        const connections = [];

        this.links.forEach(link => {
            const source = this.nodes.find(n => n.id === link.source);
            const target = this.nodes.find(n => n.id === link.target);
            if (!source || !target) return;

            // **TWO-PASS: Use confirmed directions if available, otherwise predict**
            let sourceEdge, targetEdge;

            if (link.routing) {
                // Use routing edges
                sourceEdge = link.routing.sourceEdge;
                targetEdge = link.routing.targetEdge;
            } else {
                // Fallback to prediction (ELK routing)
                sourceEdge = this.predictEdge(source, target, true);
                targetEdge = this.predictEdge(source, target, false);
            }

            // Check if this link uses the specified edge
            if (link.source === nodeId) {
                // Outgoing from this node
                if (sourceEdge === edge) {
                    connections.push({
                        link,
                        isSource: true,
                        sourceNode: source,
                        targetNode: target,
                        otherNode: target
                    });
                }
            } else if (link.target === nodeId) {
                // Incoming to this node
                if (targetEdge === edge) {
                    connections.push({
                        link,
                        isSource: false,
                        sourceNode: source,
                        targetNode: target,
                        otherNode: source
                    });
                }
            }
        });

        return connections;
    }

    /**
     * Estimate the actual snap position where a link departs from its source node
     * This is used for sorting to minimize crossings
     *
     * @param {string} sourceId - Source node ID
     * @param {string} targetId - Target node ID
     * @param {Object} link - The link object
     * @returns {number|null} Estimated X or Y coordinate, or null if cannot estimate
     */
    estimateSourceSnapPosition(sourceId, targetId, link) {
        const source = this.nodes.find(n => n.id === sourceId);
        const target = this.nodes.find(n => n.id === targetId);
        if (!source || !target) return null;

        // **TWO-PASS: Use routing edges if available**
        let sourceEdge;
        if (link.routing) {
            // Use routing edges
            sourceEdge = link.routing.sourceEdge;
        } else {
            // Fallback to prediction
            sourceEdge = this.predictEdge(source, target, true);
        }

        // Count ALL connections (incoming + outgoing) on that edge
        const connections = this.countConnectionsOnEdge(sourceId, sourceEdge);

        if (connections.length <= 1) {
            // Single connection - use node center
            return (sourceEdge === 'top' || sourceEdge === 'bottom') ? source.x : source.y;
        }

        // Sort connections to match snap position calculation
        if (sourceEdge === 'top' || sourceEdge === 'bottom') {
            // Horizontal edge: sort by other node's X
            connections.sort((a, b) => {
                const aX = a.otherNode.x || 0;
                const bX = b.otherNode.x || 0;
                return aX - bX;
            });

            // Find index of current link
            const index = connections.findIndex(c => c.link.id === link.id);
            if (index >= 0) {
                const position = (index + 1) / (connections.length + 1);
                const { halfWidth } = TransitionLayoutOptimizer.getNodeSize(source);
                return (source.x - halfWidth) + (halfWidth * 2 * position);
            }
        } else {
            // Vertical edge: sort by other node's Y
            connections.sort((a, b) => {
                const aY = a.otherNode.y || 0;
                const bY = b.otherNode.y || 0;
                return aY - bY;
            });

            // Find index of current link
            const index = connections.findIndex(c => c.link.id === link.id);
            if (index >= 0) {
                const position = (index + 1) / (connections.length + 1);
                const { halfHeight } = TransitionLayoutOptimizer.getNodeSize(source);
                return (source.y - halfHeight) + (halfHeight * 2 * position);
            }
        }

        // Fallback to node center
        return (sourceEdge === 'top' || sourceEdge === 'bottom') ? source.x : source.y;
    }

    /**
     * Calculate snap position for a link on a specific node edge
     *
     * @param {string} nodeId - Node ID
     * @param {string} edge - Edge name ('top', 'bottom', 'left', 'right')
     * @param {string} linkId - Link ID
     * @param {string} direction - Direction string (e.g., 'from-top', 'to-bottom')
     * @returns {Object|null} {x, y, index, count} or null if not applicable
     */
    calculateSnapPosition(nodeId, edge, linkId, direction) {
        const node = this.nodes.find(n => n.id === nodeId);
        if (!node) return null;

        const link = this.links.find(l => l.id === linkId);
        if (!link) return null;

        // Special case: Initial pseudo-node transitions always use center position
        if (link.linkType === 'initial') {
            const cx = node.x || 0;
            const cy = node.y || 0;

            // Get size from node object (uses actual dimensions if available)
            const { halfWidth, halfHeight } = TransitionLayoutOptimizer.getNodeSize(node);

            console.log(`[SNAP INITIAL] ${nodeId} ${edge}: ${link.source}→${link.target} (INITIAL) type=${node.type}, size=${halfWidth}x${halfHeight}`);

            if (edge === 'top') {
                return { x: cx, y: cy - halfHeight, index: 0, count: 1 };
            } else if (edge === 'bottom') {
                return { x: cx, y: cy + halfHeight, index: 0, count: 1 };
            } else if (edge === 'left') {
                return { x: cx - halfWidth, y: cy, index: 0, count: 1 };
            } else if (edge === 'right') {
                return { x: cx + halfWidth, y: cy, index: 0, count: 1 };
            }
        }

        // Block other transitions from using an edge that has an initial transition
        if (this.hasInitialTransitionOnEdge(nodeId, edge)) {
            console.log(`[SNAP BLOCKED] ${nodeId} ${edge}: ${link.source}→${link.target} blocked - initial transition owns this edge`);
            return null;  // Force fallback to different edge or center
        }

        // Get all connections on this edge
        const allConnections = this.countConnectionsOnEdge(nodeId, edge);

        if (allConnections.length === 0) return null;

        // **CRITICAL: Separate incoming and outgoing to prevent overlap**
        const incomingConns = allConnections.filter(c => !c.isSource);
        const outgoingConns = allConnections.filter(c => c.isSource);

        console.log(`[SNAP SEPARATE] ${nodeId} ${edge}: ${incomingConns.length} incoming, ${outgoingConns.length} outgoing`);

        // Sort each group independently
        const sortByPosition = (a, b, isHorizontal) => {
            let aPos, bPos;

            if (a.isSource) {
                aPos = isHorizontal ? (a.otherNode.x || 0) : (a.otherNode.y || 0);
            } else {
                aPos = this.estimateSourceSnapPosition(a.link.source, a.link.target, a.link);
                if (aPos === null) {
                    aPos = isHorizontal ? (a.otherNode.x || 0) : (a.otherNode.y || 0);
                }
            }

            if (b.isSource) {
                bPos = isHorizontal ? (b.otherNode.x || 0) : (b.otherNode.y || 0);
            } else {
                bPos = this.estimateSourceSnapPosition(b.link.source, b.link.target, b.link);
                if (bPos === null) {
                    bPos = isHorizontal ? (b.otherNode.x || 0) : (b.otherNode.y || 0);
                }
            }

            return aPos - bPos;
        };

        const isHorizontal = (edge === 'top' || edge === 'bottom');
        incomingConns.sort((a, b) => sortByPosition(a, b, isHorizontal));
        outgoingConns.sort((a, b) => sortByPosition(a, b, isHorizontal));

        // Determine which group this link belongs to and find its index
        const isIncoming = (link.target === nodeId);
        const group = isIncoming ? incomingConns : outgoingConns;
        const groupIndex = group.findIndex(c => c.link.id === linkId);

        if (groupIndex < 0) {
            console.error(`[SNAP ERROR] Link ${linkId} not found in its group`);
            return null;
        }

        // **Calculate position ensuring no overlap between incoming and outgoing**
        // Total slots: incomingConns.length + outgoingConns.length
        // Incoming: positions 1, 2, ..., incomingConns.length
        // Outgoing: positions incomingConns.length+1, ..., total
        const totalCount = incomingConns.length + outgoingConns.length;
        let absoluteIndex;

        if (isIncoming) {
            absoluteIndex = groupIndex;
        } else {
            absoluteIndex = incomingConns.length + groupIndex;
        }

        const position = (absoluteIndex + 1) / (totalCount + 1);

        console.log(`[SNAP] ${nodeId} ${edge}: ${link.source}→${link.target} (${isIncoming ? 'IN' : 'OUT'}) at ${groupIndex + 1}/${group.length} in group, absolute ${absoluteIndex + 1}/${totalCount}, position ${position.toFixed(3)}`);

        // Calculate actual coordinates
        const cx = node.x || 0;
        const cy = node.y || 0;
        const { halfWidth, halfHeight } = TransitionLayoutOptimizer.getNodeSize(node);

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

        return { x, y, index: absoluteIndex, count: totalCount };
    }

    /**
     * Get all possible snap point combinations for a link
     * @param {Object} link - Link object
     * @param {Object} sourceNode - Source node
     * @param {Object} targetNode - Target node
     * @param {Object} reverseRouting - Optional: routing of reverse link (for bidirectional transitions)
     * @returns {Array} Array of {sourceEdge, targetEdge, sourcePoint, targetPoint, distance}
     */
    getAllPossibleSnapCombinations(link, sourceNode, targetNode, reverseRouting = null) {
        const combinations = [];
        const edges = ['top', 'bottom', 'left', 'right'];

        edges.forEach(sourceEdge => {
            // Skip if source edge has initial transition blocking
            if (link.linkType !== 'initial' &&
                this.hasInitialTransitionOnEdge(sourceNode.id, sourceEdge)) {
                return;
            }

            edges.forEach(targetEdge => {
                // Skip if target edge has initial transition blocking
                if (link.linkType !== 'initial' &&
                    this.hasInitialTransitionOnEdge(targetNode.id, targetEdge)) {
                    return;
                }

                // **BIDIRECTIONAL CONSTRAINT**: Skip if reverse link uses same edge pair in opposite direction
                if (reverseRouting) {
                    // Check if current combination is reverse of the assigned routing
                    // Example: if reverse is "on.bottom → off.right", skip "off.right → on.bottom"
                    if (sourceEdge === reverseRouting.targetEdge &&
                        targetEdge === reverseRouting.sourceEdge) {
                        console.log(`[BIDIRECTIONAL SKIP] ${link.source}.${sourceEdge}→${link.target}.${targetEdge} (reverse already uses this edge pair)`);
                        return;
                    }
                }

                // Calculate snap points for this edge combination
                const sourcePoint = this.getEdgeCenterPoint(sourceNode, sourceEdge);
                const targetPoint = this.getEdgeCenterPoint(targetNode, targetEdge);

                // Calculate distance
                const dx = targetPoint.x - sourcePoint.x;
                const dy = targetPoint.y - sourcePoint.y;
                const distance = Math.sqrt(dx * dx + dy * dy);

                combinations.push({
                    sourceEdge,
                    targetEdge,
                    sourcePoint,
                    targetPoint,
                    distance
                });
            });
        });

        return combinations;
    }

    /**
     * Get center point of a node edge
     * @param {Object} node - Node object
     * @param {string} edge - Edge name
     * @returns {Object} {x, y}
     */
    getEdgeCenterPoint(node, edge) {
        const cx = node.x || 0;
        const cy = node.y || 0;
        const { halfWidth, halfHeight } = TransitionLayoutOptimizer.getNodeSize(node);

        if (edge === 'top') {
            return { x: cx, y: cy - halfHeight };
        } else if (edge === 'bottom') {
            return { x: cx, y: cy + halfHeight };
        } else if (edge === 'left') {
            return { x: cx - halfWidth, y: cy };
        } else if (edge === 'right') {
            return { x: cx + halfWidth, y: cy };
        }
    }

    /**
     * Calculate number of intersections between two paths
     * Uses orthogonal path assumption (Z-shaped or direct)
     * @param {Object} path1 - {sourcePoint, targetPoint}
     * @param {Object} path2 - {sourcePoint, targetPoint}
     * @returns {number} Number of intersections
     */
    calculatePathIntersections(path1, path2) {
        const segments1 = this.getPathSegments(path1);
        const segments2 = this.getPathSegments(path2);

        let intersections = 0;

        segments1.forEach(seg1 => {
            segments2.forEach(seg2 => {
                if (this.segmentsIntersect(seg1, seg2)) {
                    intersections++;
                }
            });
        });

        return intersections;
    }

    /**
     * Convert path to line segments (orthogonal)
     * @param {Object} path - {sourcePoint, targetPoint}
     * @returns {Array} Array of {x1, y1, x2, y2}
     */
    getPathSegments(path) {
        const sx = path.sourcePoint.x;
        const sy = path.sourcePoint.y;
        const tx = path.targetPoint.x;
        const ty = path.targetPoint.y;
        const sourceEdge = path.sourceEdge;
        const targetEdge = path.targetEdge;

        const segments = [];
        const MIN_SEGMENT = TransitionLayoutOptimizer.MIN_SEGMENT_LENGTH;

        // Check if direct line (horizontal or vertical alignment)
        const dx = Math.abs(tx - sx);
        const dy = Math.abs(ty - sy);

        if (dx < 1 || dy < 1) {
            // Direct line
            segments.push({ x1: sx, y1: sy, x2: tx, y2: ty });
            return segments;
        }

        // Generate path segments matching visualizer's createOrthogonalPath logic
        const sourceIsVertical = (sourceEdge === 'top' || sourceEdge === 'bottom');
        const targetIsVertical = (targetEdge === 'top' || targetEdge === 'bottom');

        if (sourceIsVertical && targetIsVertical) {
            // Both vertical edges: vertical-horizontal-vertical (4 segments)
            let y1 = (sourceEdge === 'top') ? sy - MIN_SEGMENT : sy + MIN_SEGMENT;
            let y2 = (targetEdge === 'top') ? ty - MIN_SEGMENT : ty + MIN_SEGMENT;

            segments.push({ x1: sx, y1: sy, x2: sx, y2: y1 });        // Vertical from source
            segments.push({ x1: sx, y1: y1, x2: tx, y2: y1 });        // Horizontal
            segments.push({ x1: tx, y1: y1, x2: tx, y2: y2 });        // Vertical
            segments.push({ x1: tx, y1: y2, x2: tx, y2: ty });        // Vertical to target
        } else if (!sourceIsVertical && !targetIsVertical) {
            // Both horizontal edges: horizontal-vertical-horizontal (4 segments)
            let x1 = (sourceEdge === 'right') ? sx + MIN_SEGMENT : sx - MIN_SEGMENT;
            let x2 = (targetEdge === 'right') ? tx + MIN_SEGMENT : tx - MIN_SEGMENT;

            segments.push({ x1: sx, y1: sy, x2: x1, y2: sy });        // Horizontal from source
            segments.push({ x1: x1, y1: sy, x2: x1, y2: ty });        // Vertical
            segments.push({ x1: x1, y1: ty, x2: x2, y2: ty });        // Horizontal
            segments.push({ x1: x2, y1: ty, x2: tx, y2: ty });        // Horizontal to target
        } else if (sourceIsVertical && !targetIsVertical) {
            // Source vertical, target horizontal (3 segments)
            let y1 = (sourceEdge === 'top') ? sy - MIN_SEGMENT : sy + MIN_SEGMENT;
            let x2 = (targetEdge === 'right') ? tx + MIN_SEGMENT : tx - MIN_SEGMENT;

            segments.push({ x1: sx, y1: sy, x2: sx, y2: y1 });        // Vertical from source
            segments.push({ x1: sx, y1: y1, x2: x2, y2: y1 });        // Horizontal
            segments.push({ x1: x2, y1: y1, x2: x2, y2: ty });        // Vertical
            segments.push({ x1: x2, y1: ty, x2: tx, y2: ty });        // Horizontal to target
        } else {
            // Source horizontal, target vertical (3 segments)
            let x1 = (sourceEdge === 'right') ? sx + MIN_SEGMENT : sx - MIN_SEGMENT;
            let y2 = (targetEdge === 'top') ? ty - MIN_SEGMENT : ty + MIN_SEGMENT;

            segments.push({ x1: sx, y1: sy, x2: x1, y2: sy });        // Horizontal from source
            segments.push({ x1: x1, y1: sy, x2: x1, y2: y2 });        // Vertical
            segments.push({ x1: x1, y1: y2, x2: tx, y2: y2 });        // Horizontal
            segments.push({ x1: tx, y1: y2, x2: tx, y2: ty });        // Vertical to target
        }

        return segments;
    }

    /**
     * Check if two line segments intersect
     * @param {Object} seg1 - {x1, y1, x2, y2}
     * @param {Object} seg2 - {x1, y1, x2, y2}
     * @returns {boolean} True if segments intersect
     */
    segmentsIntersect(seg1, seg2) {
        const ccw = (A, B, C) => {
            return (C.y - A.y) * (B.x - A.x) > (B.y - A.y) * (C.x - A.x);
        };

        const A = { x: seg1.x1, y: seg1.y1 };
        const B = { x: seg1.x2, y: seg1.y2 };
        const C = { x: seg2.x1, y: seg2.y1 };
        const D = { x: seg2.x2, y: seg2.y2 };

        // Check if endpoints are the same (touching, not crossing)
        const touching = (
            (Math.abs(A.x - C.x) < 0.1 && Math.abs(A.y - C.y) < 0.1) ||
            (Math.abs(A.x - D.x) < 0.1 && Math.abs(A.y - D.y) < 0.1) ||
            (Math.abs(B.x - C.x) < 0.1 && Math.abs(B.y - C.y) < 0.1) ||
            (Math.abs(B.x - D.x) < 0.1 && Math.abs(B.y - D.y) < 0.1)
        );

        if (touching) return false;

        return ccw(A, C, D) !== ccw(B, C, D) && ccw(A, B, C) !== ccw(A, B, D);
    }

    /**
     * Check if a path intersects with a node
     * @param {Object} path - {sourcePoint, targetPoint}
     * @param {Object} node - Node object
     * @param {Object} options - { skipFirstSegment: boolean, skipLastSegment: boolean }
     * @returns {boolean} True if path intersects node
     */
    pathIntersectsNode(path, node, options = {}) {
        const { skipFirstSegment = false, skipLastSegment = false } = options;
        const { halfWidth, halfHeight } = TransitionLayoutOptimizer.getNodeSize(node);

        const nodeLeft = node.x - halfWidth;
        const nodeRight = node.x + halfWidth;
        const nodeTop = node.y - halfHeight;
        const nodeBottom = node.y + halfHeight;

        const segments = this.getPathSegments(path);

        for (let i = 0; i < segments.length; i++) {
            const segment = segments[i];

            // Skip first segment if requested (for source node collision check)
            if (skipFirstSegment && i === 0) continue;

            // Skip last segment if requested (for target node collision check)
            if (skipLastSegment && i === segments.length - 1) {
                // Special case: Direct line (single segment)
                // Don't skip the entire segment, only exclude the endpoint area
                if (segments.length === 1) {
                    log(`[PATH-NODE] Direct line for ${node.id}: checking segment except endpoint`);
                    // Check segment but exclude endpoint (target point) area
                    if (this.segmentIntersectsRectExcludingEndpoint(segment, nodeLeft, nodeTop, nodeRight, nodeBottom, path.targetPoint)) {
                        log(`[PATH-NODE] Direct line segment intersects ${node.id} (excluding endpoint)`);
                        return true;
                    }
                    continue; // Skip normal check
                } else {
                    log(`[PATH-NODE] Skipping last segment for ${node.id}: segment ${i}/${segments.length-1}`);
                    continue;
                }
            }

            // Check if segment intersects with node bounding box
            if (this.segmentIntersectsRect(segment, nodeLeft, nodeTop, nodeRight, nodeBottom)) {
                log(`[PATH-NODE] Segment ${i} intersects ${node.id}: (${segment.x1.toFixed(1)},${segment.y1.toFixed(1)})→(${segment.x2.toFixed(1)},${segment.y2.toFixed(1)})`);
                return true;
            }
        }

        return false;
    }

    /**
     * Check if a line segment intersects with a rectangle
     * @param {Object} segment - {x1, y1, x2, y2}
     * @param {number} left - Rectangle left boundary
     * @param {number} top - Rectangle top boundary
     * @param {number} right - Rectangle right boundary
     * @param {number} bottom - Rectangle bottom boundary
     * @returns {boolean} True if segment intersects rectangle
     */
    segmentIntersectsRect(segment, left, top, right, bottom) {
        const { x1, y1, x2, y2 } = segment;

        // Check if segment is completely inside the rectangle
        const p1Inside = (x1 >= left && x1 <= right && y1 >= top && y1 <= bottom);
        const p2Inside = (x2 >= left && x2 <= right && y2 >= top && y2 <= bottom);

        if (p1Inside || p2Inside) {
            return true;
        }

        // Check if segment intersects any of the rectangle edges
        const rectSegments = [
            { x1: left, y1: top, x2: right, y2: top },       // Top edge
            { x1: right, y1: top, x2: right, y2: bottom },   // Right edge
            { x1: left, y1: bottom, x2: right, y2: bottom }, // Bottom edge
            { x1: left, y1: top, x2: left, y2: bottom }      // Left edge
        ];

        for (const rectSeg of rectSegments) {
            if (this.segmentsIntersect(segment, rectSeg)) {
                return true;
            }
        }

        return false;
    }

    /**
     * Check if a line segment intersects with a rectangle, excluding endpoint area
     * Used for direct line segments where the endpoint is the target snap point
     * @param {Object} segment - {x1, y1, x2, y2}
     * @param {number} left - Rectangle left boundary
     * @param {number} top - Rectangle top boundary
     * @param {number} right - Rectangle right boundary
     * @param {number} bottom - Rectangle bottom boundary
     * @param {Object} endpoint - {x, y} - The endpoint to exclude
     * @returns {boolean} True if segment intersects rectangle (excluding endpoint area)
     */
    segmentIntersectsRectExcludingEndpoint(segment, left, top, right, bottom, endpoint) {
        const MIN_SEGMENT = TransitionLayoutOptimizer.MIN_SEGMENT_LENGTH;
        const { x1, y1, x2, y2 } = segment;

        // Determine which end is the endpoint (usually x2, y2 for target)
        const isVertical = Math.abs(x2 - x1) < 1;
        const isHorizontal = Math.abs(y2 - y1) < 1;

        // Shorten the segment to exclude the endpoint area (MIN_SEGMENT distance)
        let shortenedSegment = { ...segment };

        if (isVertical) {
            // Vertical segment: exclude endpoint in Y direction
            if (y2 > y1) {
                // Going down
                shortenedSegment.y2 = y2 - MIN_SEGMENT;
            } else {
                // Going up
                shortenedSegment.y2 = y2 + MIN_SEGMENT;
            }
        } else if (isHorizontal) {
            // Horizontal segment: exclude endpoint in X direction
            if (x2 > x1) {
                // Going right
                shortenedSegment.x2 = x2 - MIN_SEGMENT;
            } else {
                // Going left
                shortenedSegment.x2 = x2 + MIN_SEGMENT;
            }
        }

        // Check if shortened segment intersects the rectangle
        return this.segmentIntersectsRect(shortenedSegment, left, top, right, bottom);
    }

    /**
     * Evaluate a specific edge combination for a link
     * @param {Object} link - Link object
     * @param {Object} sourceNode - Source node
     * @param {Object} targetNode - Target node
     * @param {string} sourceEdge - Source edge name
     * @param {string} targetEdge - Target edge name
     * @param {Array} assignedPaths - Already assigned paths (for intersection checking)
     * @param {Array} nodes - All nodes
     * @returns {Object} {combination, score}
     */
    evaluateCombination(link, sourceNode, targetNode, sourceEdge, targetEdge, assignedPaths, nodes) {
        const sourcePoint = this.getEdgeCenterPoint(sourceNode, sourceEdge);
        const targetPoint = this.getEdgeCenterPoint(targetNode, targetEdge);

        const dx = targetPoint.x - sourcePoint.x;
        const dy = targetPoint.y - sourcePoint.y;
        const distance = Math.sqrt(dx * dx + dy * dy);

        const combo = {
            sourceEdge,
            targetEdge,
            sourcePoint,
            targetPoint,
            distance
        };

        // Calculate intersections
        let intersections = 0;
        assignedPaths.forEach(assignedPath => {
            intersections += this.calculatePathIntersections(combo, assignedPath);
        });

        // Calculate node collisions
        let nodeCollisions = 0;

        if (this.pathIntersectsNode(combo, sourceNode, { skipFirstSegment: true })) {
            nodeCollisions++;
        }
        if (this.pathIntersectsNode(combo, targetNode, { skipLastSegment: true })) {
            nodeCollisions++;
        }
        nodes.forEach(node => {
            if (node.id === sourceNode.id || node.id === targetNode.id) return;
            if (this.pathIntersectsNode(combo, node)) {
                nodeCollisions++;
            }
        });

        // Check too-close snap
        const MIN_SAFE_DISTANCE = TransitionLayoutOptimizer.MIN_SAFE_DISTANCE;
        let tooCloseSnap = 0;
        const sourceIsVertical = (sourceEdge === 'top' || sourceEdge === 'bottom');
        const targetIsVertical = (targetEdge === 'top' || targetEdge === 'bottom');
        const distX = Math.abs(sourcePoint.x - targetPoint.x);
        const distY = Math.abs(sourcePoint.y - targetPoint.y);

        // For horizontal edges (left/right), check if BOTH x and y are too close
        // This allows vertical layouts where x is close but y is far
        if (!sourceIsVertical && !targetIsVertical) {
            if (distX < MIN_SAFE_DISTANCE && distY < MIN_SAFE_DISTANCE) {
                tooCloseSnap = 1;
            }
        }
        // For vertical edges (top/bottom), check if BOTH x and y are too close
        else if (sourceIsVertical && targetIsVertical) {
            if (distY < MIN_SAFE_DISTANCE && distX < MIN_SAFE_DISTANCE) {
                tooCloseSnap = 1;
            }
        }

        const score = tooCloseSnap * 100000 + nodeCollisions * 10000 + intersections * 1000 + distance;

        return { combination: combo, score };
    }

    /**
     * Sort links with initial transitions first
     * @private
     * @param {Array} links - Array of link objects
     * @returns {Array} Sorted links array
     */
    _sortLinksByPriority(links) {
        return [...links].sort((a, b) => {
            if (a.linkType === 'initial' && b.linkType !== 'initial') return -1;
            if (a.linkType !== 'initial' && b.linkType === 'initial') return 1;
            return 0;
        });
    }

    /**
     * Apply CSP solution to links and redistribute snap points
     * @private
     * @param {Map|Array} assignment - CSP assignment (Map or Array format)
     * @param {Array} links - Array of link objects
     * @param {Array} nodes - Array of node objects
     */
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

    /**
     * Handle progress message from worker
     * @private
     * @param {Object} data - Message data from worker
     * @param {Array} links - Array of link objects
     * @param {Array} nodes - Array of node objects
     * @param {number} lastProgressRender - Last render timestamp
     * @returns {number} Updated lastProgressRender timestamp
     */
    _handleProgressMessage(data, links, nodes, lastProgressRender) {
        if (data.data && data.data.type === 'solution_improved') {
            const now = performance.now();
            if (now - lastProgressRender >= TransitionLayoutOptimizer.PROGRESS_RENDER_INTERVAL_MS) {
                console.log(`[OPTIMIZE PROGRESSIVE] Solution improved: score=${data.data.score.toFixed(1)}, progress=${(data.data.progress * 100).toFixed(1)}%`);
                this._applySolutionToLinks(data.data.assignment, links, nodes);
                return now;
            }
        }
        return lastProgressRender;
    }

    /**
     * Handle solution message from worker
     * @private
     * @param {Array} solution - CSP solution assignments
     * @param {number} score - Solution score
     * @param {Object} stats - Statistics (nodeCount, pruneCount)
     * @param {Array} links - Array of link objects
     * @param {Array} nodes - Array of node objects
     * @param {Worker} worker - Web Worker instance
     * @param {Function} onComplete - Completion callback
     * @returns {null} Worker is terminated, returns null
     */
    _handleSolutionMessage(solution, score, stats, links, nodes, worker, onComplete) {
        console.log(`[OPTIMIZE PROGRESSIVE] CSP solution received (score=${score}, nodes=${stats.nodeCount}, prunes=${stats.pruneCount})`);
        console.log('[OPTIMIZE PROGRESSIVE] Applying CSP solution...');
        this._applySolutionToLinks(solution, links, nodes);
        console.log('[OPTIMIZE PROGRESSIVE] Background CSP complete!');
        worker.terminate();
        if (onComplete) onComplete(true);
        return null;
    }

    /**
     * Handle cancelled message from worker
     * @private
     * @param {Worker} worker - Web Worker instance
     * @param {Function} onComplete - Completion callback
     * @returns {null} Worker is terminated, returns null
     */
    _handleCancelledMessage(worker, onComplete) {
        console.log('[OPTIMIZE PROGRESSIVE] CSP cancelled by user');
        worker.terminate();
        if (onComplete) onComplete(false);
        return null;
    }

    /**
     * Handle failed message from worker
     * @private
     * @param {Worker} worker - Web Worker instance
     * @param {Function} onComplete - Completion callback
     * @returns {null} Worker is terminated, returns null
     */
    _handleFailedMessage(worker, onComplete) {
        console.log('[OPTIMIZE PROGRESSIVE] CSP failed, keeping greedy result');
        worker.terminate();
        if (onComplete) onComplete(false);
        return null;
    }

    /**
     * Handle error message from worker
     * @private
     * @param {string} message - Error message
     * @param {string} stack - Error stack trace
     * @param {Worker} worker - Web Worker instance
     * @param {Function} onComplete - Completion callback
     * @returns {null} Worker is terminated, returns null
     */
    _handleErrorMessage(message, stack, worker, onComplete) {
        console.error('[OPTIMIZE PROGRESSIVE] Worker error:', message);
        console.error(stack);
        worker.terminate();
        if (onComplete) onComplete(false);
        return null;
    }

    /**
     * Progressive optimization: immediate greedy + background CSP
     * @param {Array} links - Array of link objects
     * @param {Array} nodes - Array of node objects
     * @param {string|null} draggedNodeId - ID of dragged node
     * @param {Function} onComplete - Callback when CSP completes: (success) => void
     * @param {Function} onProgress - Callback for intermediate updates: (iteration, totalIterations, score) => void
     * @param {number} debounceMs - Delay before starting CSP (default: 500ms)
     * @returns {Object} { cancel: () => void } - Object with cancel method
     */
    optimizeSnapPointAssignmentsProgressive(links, nodes, draggedNodeId, onComplete, onProgress = null, debounceMs = 500) {
        // Filter out containment and delegation links (only optimize transition and initial links)
        // Visualizer layout: containment is hierarchical structure, not routing path
        const transitionLinks = links.filter(link => 
            link.linkType === 'transition' || link.linkType === 'initial'
        );

        console.log('[OPTIMIZE PROGRESSIVE] Starting progressive optimization...');

        // Progressive optimization: greedy step for immediate feedback
        console.log('[OPTIMIZE PROGRESSIVE] Greedy step: immediate rendering...');
        this.optimizeSnapPointAssignmentsGreedy(transitionLinks, nodes, draggedNodeId);

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
                            this._applySolutionToLinks(solution, transitionLinks, nodes);

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
                const greedySolution = this.convertGreedyToCSPSolution(transitionLinks);

                // Build parent-child map
                const parentChildMap = {};
                this.links.forEach(link => {
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

    /**
     * Fallback to main thread CSP when Worker is not available
     * @private
     */
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
            this.links.forEach(link => {
                if (link.linkType === 'containment') {
                    parentChildMap.set(link.target, link.source);
                }
            });

            // Convert greedy results to CSP solution format
            let currentBestSolution = this.convertGreedyToCSPSolution(links);
            console.log(`[OPTIMIZE PROGRESSIVE] Initial solution: score=${currentBestSolution.score.toFixed(1)}`);

            // Progressive refinement: 8 iterations × 250ms = 2000ms total
            const MAX_ITERATIONS = 8;
            let bestScoreOverall = currentBestSolution.score;
            let bestSolutionOverall = null;

            for (let iteration = 0; iteration < MAX_ITERATIONS; iteration++) {
                console.log(`[OPTIMIZE PROGRESSIVE] === Iteration ${iteration + 1}/${MAX_ITERATIONS} ===`);

                // Create CSP solver with current best solution as warm-start
                const solver = new ConstraintSolver(links, nodes, this, parentChildMap, currentBestSolution, draggedNodeId);

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
                    this.distributeSnapPointsOnEdges(sortedLinks, nodes);

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

    /**
     * Optimize snap point assignments for all links
     * Uses Constraint Satisfaction Problem (CSP) solver for globally optimal routing
     * Algorithm steps:
     * 1. CSP solver assigns optimal edges (hard constraints + soft constraints)
     * 2. Distribute snap points on each edge to minimize congestion
     * @param {Array} links - Array of link objects
     * @param {Array} nodes - Array of node objects
     * @param {boolean} useGreedy - Use greedy algorithm instead of CSP
     * @param {string} draggedNodeId - ID of dragged node for locality-aware optimization (optional)
     */
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
            this.optimizeSnapPointAssignmentsGreedy(transitionLinks, nodes, draggedNodeId);
            return;
        }

        // Progressive Enhancement Strategy:
        // 1. First: Apply Greedy algorithm immediately (fast, O(n))
        // 2. Then: Run CSP solver in background (slow, optimal)
        // 3. Update layout if CSP finds better solution

        console.log(`[OPTIMIZE] Progressive optimization for ${transitionLinks.length} transitions...`);
        console.log('[OPTIMIZE GREEDY] Applying Greedy algorithm immediately...');

        // Apply Greedy first for instant feedback
        this.optimizeSnapPointAssignmentsGreedy(transitionLinks, nodes, draggedNodeId);
        
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
        if (this.cspRunning) {
            console.log('[OPTIMIZE CSP] CSP already running, skipping duplicate optimization');
            return;
        }

        console.log('[OPTIMIZE CSP] Starting background CSP optimization...');
        
        // Set flag IMMEDIATELY to prevent duplicate scheduling
        // (requestIdleCallback has delay between schedule and execution)
        this.cspRunning = true;
        
        // Use requestIdleCallback for truly non-blocking execution
        // CSP will only run when browser is idle (not blocking user interactions)
        const scheduleCSP = () => {
            if (typeof requestIdleCallback !== 'undefined') {
                requestIdleCallback(() => {
                    this.runBackgroundCSPOptimization(transitionLinks, nodes, draggedNodeId);
                }, { timeout: 5000 }); // Fallback to setTimeout after 5s
            } else {
                // Fallback for browsers without requestIdleCallback
                setTimeout(() => {
                    this.runBackgroundCSPOptimization(transitionLinks, nodes, draggedNodeId);
                }, 100); // Small delay to let UI render
            }
        };
        
        scheduleCSP();
    }

    /**
     * Convert greedy routing results to CSP solution format
     * @param {Array} links - Links with routing assigned by greedy
     * @returns {Object} {assignment: Map, score: number, preferences: Map}
     */
    convertGreedyToCSPSolution(links) {
        const assignment = new Map();
        const preferences = new Map();
        let totalScore = 0;

        links.forEach(link => {
            if (!link.routing) return;

            const sourceNode = this.nodes.find(n => n.id === link.source);
            const targetNode = this.nodes.find(n => n.id === link.target);
            if (!sourceNode || !targetNode) return;

            const combo = {
                sourceEdge: link.routing.sourceEdge,
                targetEdge: link.routing.targetEdge,
                sourcePoint: this.getEdgeCenterPoint(sourceNode, link.routing.sourceEdge),
                targetPoint: this.getEdgeCenterPoint(targetNode, link.routing.targetEdge)
            };

            const dx = combo.targetPoint.x - combo.sourcePoint.x;
            const dy = combo.targetPoint.y - combo.sourcePoint.y;
            combo.distance = Math.sqrt(dx * dx + dy * dy);

            // Calculate score (use same logic as greedy for consistency)
            const score = this.calculateComboScore(link, combo, assignment, links);

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

    /**
     * Calculate score for a combo (shared by greedy and CSP)
     * @private
     */
    calculateComboScore(link, combo, existingAssignment, allLinks) {
        // Calculate intersections with existing assignments
        let intersections = 0;
        for (const [assignedLinkId, assignedData] of existingAssignment) {
            intersections += this.calculatePathIntersections(combo, assignedData.combo);
        }

        // Calculate node collisions
        let nodeCollisions = 0;
        const sourceNode = this.nodes.find(n => n.id === link.source);
        const targetNode = this.nodes.find(n => n.id === link.target);

        if (sourceNode && this.pathIntersectsNode(combo, sourceNode, { skipFirstSegment: true })) {
            nodeCollisions++;
        }
        if (targetNode && this.pathIntersectsNode(combo, targetNode, { skipLastSegment: true })) {
            nodeCollisions++;
        }

        this.nodes.forEach(node => {
            if (node.id === link.source || node.id === link.target) return;
            if (this.pathIntersectsNode(combo, node)) {
                nodeCollisions++;
            }
        });

        // Check too-close snap points
        const MIN_SAFE_DISTANCE = TransitionLayoutOptimizer.MIN_SAFE_DISTANCE;
        let tooCloseSnap = 0;
        const sourceIsVertical = (combo.sourceEdge === 'top' || combo.sourceEdge === 'bottom');
        const targetIsVertical = (combo.targetEdge === 'top' || combo.targetEdge === 'bottom');
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

        // Check self-overlap
        const MIN_SEGMENT = TransitionLayoutOptimizer.MIN_SEGMENT_LENGTH;
        let selfOverlap = 0;

        if (!sourceIsVertical && !targetIsVertical) {
            let x1 = (combo.sourceEdge === 'right') ? combo.sourcePoint.x + MIN_SEGMENT : combo.sourcePoint.x - MIN_SEGMENT;
            let x2 = (combo.targetEdge === 'right') ? combo.targetPoint.x + MIN_SEGMENT : combo.targetPoint.x - MIN_SEGMENT;
            if (combo.targetEdge === 'left' && x1 > x2) selfOverlap = 1;
            if (combo.targetEdge === 'right' && x1 < x2) selfOverlap = 1;
        } else if (sourceIsVertical && targetIsVertical) {
            let y1 = (combo.sourceEdge === 'top') ? combo.sourcePoint.y - MIN_SEGMENT : combo.sourcePoint.y + MIN_SEGMENT;
            let y2 = (combo.targetEdge === 'top') ? combo.targetPoint.y - MIN_SEGMENT : combo.targetPoint.y + MIN_SEGMENT;
            if (combo.targetEdge === 'top' && y1 > y2) selfOverlap = 1;
            if (combo.targetEdge === 'bottom' && y1 < y2) selfOverlap = 1;
        }

        return tooCloseSnap * 100000 + selfOverlap * 50000 + nodeCollisions * 10000 + intersections * 1000 + combo.distance;
    }

    /**
     * Run CSP solver in background and update layout if better solution found
     * Called asynchronously after Greedy layout is applied
     */
    runBackgroundCSPOptimization(transitionLinks, nodes, draggedNodeId) {
        // Note: cspRunning flag already set in optimizeSnapPointAssignments()
        // to prevent duplicate scheduling before requestIdleCallback executes
        
        console.log('[CSP BACKGROUND] Starting CSP solver (non-blocking, max 500ms)...');
        const startTime = performance.now();

        // Build parent-child map from containment links
        const parentChildMap = new Map(); // Map<childId, parentId>
        this.links.forEach(link => {
            if (link.linkType === 'containment') {
                parentChildMap.set(link.target, link.source);
            }
        });
        
        console.log(`[CSP BACKGROUND] Built parent-child map with ${parentChildMap.size} entries`);
        for (const [child, parent] of parentChildMap.entries()) {
            console.log(`  ${parent} → ${child}`);
        }

        // Convert greedy results to CSP solution format (Strategy 1 + 3)
        const greedySolution = this.convertGreedyToCSPSolution(transitionLinks);
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
        const solver = new ConstraintSolver(transitionLinks, nodes, this, parentChildMap, greedySolution, draggedNodeId);

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
            this.cspRunning = false;
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

                console.log(`[CSP BACKGROUND UPDATE] ${link.source}→${link.target}: ${assignment.sourceEdge}→${assignment.targetEdge}`);
            }
        });

        // Distribute snap points on each edge to minimize congestion
        const sortedLinks = [...transitionLinks].sort((a, b) => {
            if (a.linkType === 'initial' && b.linkType !== 'initial') return -1;
            if (a.linkType !== 'initial' && b.linkType === 'initial') return 1;
            return 0;
        });
        this.distributeSnapPointsOnEdges(sortedLinks, nodes);

        console.log('[CSP BACKGROUND] Layout updated with CSP solution');
        
        // Trigger re-render if callback exists
        if (this.onLayoutUpdated) {
            console.log('[CSP BACKGROUND] Triggering re-render...');
            this.onLayoutUpdated();
        }

        // Clear flag
        this.cspRunning = false;
    }

    /**
     * Fallback greedy algorithm (edge assignment + snap distribution)
     * Used when CSP solver is not available or fails
     * @param {Array} links - Array of link objects
     * @param {Array} nodes - Array of node objects
     * @param {string|null} draggedNodeId - ID of dragged node (for caching optimization)
     */
    optimizeSnapPointAssignmentsGreedy(links, nodes, draggedNodeId = null) {
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
            const key = `${link.source}→${link.target}`;
            reverseLinkMap.set(key, link);
        });

        // Assign optimal edges to each link (greedy selection)
        sortedLinks.forEach(link => {
            const sourceNode = nodes.find(n => n.id === link.source);
            const targetNode = nodes.find(n => n.id === link.target);

            if (!sourceNode || !targetNode) return;

            // OPTIMIZATION: Check if link is affected by drag
            let isAffectedByDrag = true; // Default: recalculate everything

            if (draggedNodeId && link.routing) {
                // Has dragged node and link has existing routing (from CSP)

                // Direct connection to dragged node?
                const isDirectlyConnected = (link.source === draggedNodeId || link.target === draggedNodeId);

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
                const cachedCombo = this.getAllPossibleSnapCombinations(link, sourceNode, targetNode, null)
                    .find(combo => combo.sourceEdge === link.routing.sourceEdge &&
                                   combo.targetEdge === link.routing.targetEdge);

                if (cachedCombo) {
                    assignedPaths.push(cachedCombo);
                    cachedCount++;
                    console.log(`[OPTIMIZE GREEDY CACHE] ${link.source}→${link.target}: keeping CSP routing ${link.routing.sourceEdge}→${link.routing.targetEdge}`);
                    return; // Skip recalculation
                }
            }

            // Link affected by drag: recalculate
            recalculatedCount++;

            // Check if reverse link exists and has routing assigned
            const reverseKey = `${link.target}→${link.source}`;
            const reverseLink = reverseLinkMap.get(reverseKey);
            const reverseRouting = (reverseLink && reverseLink.routing) ? reverseLink.routing : null;

            if (reverseRouting) {
                console.log(`[BIDIRECTIONAL DETECT] ${link.source}→${link.target} has reverse link with routing: ${reverseRouting.sourceEdge}→${reverseRouting.targetEdge}`);
            }

            // Get all possible combinations (exclude reverse edge pair if bidirectional)
            const combinations = this.getAllPossibleSnapCombinations(link, sourceNode, targetNode, reverseRouting);

            if (combinations.length === 0) {
                console.warn(`No valid snap combinations for ${link.source}→${link.target}`);
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
                    intersections += this.calculatePathIntersections(combo, assignedPath);
                });

                // Calculate node collisions
                let nodeCollisions = 0;

                if (this.pathIntersectsNode(combo, sourceNode, { skipFirstSegment: true })) {
                    nodeCollisions++;
                }

                if (this.pathIntersectsNode(combo, targetNode, { skipLastSegment: true })) {
                    nodeCollisions++;
                }

                nodes.forEach(node => {
                    if (node.id === sourceNode.id || node.id === targetNode.id) return;

                    if (this.pathIntersectsNode(combo, node)) {
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

                console.log(`[OPTIMIZE GREEDY] ${link.source}→${link.target}: ${bestCombination.sourceEdge}→${bestCombination.targetEdge}, score=${bestScore.toFixed(1)}`);
            }
        });

        // Distribute snap points on each edge to minimize congestion
        this.distributeSnapPointsOnEdges(sortedLinks, nodes);

        console.log(`[OPTIMIZE GREEDY] Completed: ${links.length} links (cached: ${cachedCount}, recalculated: ${recalculatedCount})`);
    }

    /**
     * Distribute snap points evenly on each edge
     * @param {Array} links - Array of link objects
     * @param {Array} nodes - Array of node objects
     */
    distributeSnapPointsOnEdges(links, nodes) {
        // Group links by node and edge (combine incoming and outgoing)
        // Note: Expects already filtered links (transition and initial only)
        const edgeGroups = new Map(); // Key: "nodeId:edge", Value: [links]

        links.forEach(link => {
            // Check routing
            if (!link.routing) return;

            // **SPECIAL: Initial transitions start from center, not edge**
            if (link.linkType === 'initial') {
                const sourceNode = nodes.find(n => n.id === link.source);
                if (sourceNode && sourceNode.type === 'initial-pseudo') {
                    // Use center of initial pseudo-node
                    const centerPoint = {
                        x: sourceNode.x || 0,
                        y: sourceNode.y || 0
                    };

                    if (link.routing) {
                        link.routing.sourcePoint = centerPoint;
                    }
                    console.log(`[OPTIMIZE CSP] ${link.source}→${link.target} source at initial center: (${centerPoint.x.toFixed(1)}, ${centerPoint.y.toFixed(1)})`);
                }

                // Still need to add target to edge group
                const targetKey = `${link.target}:${link.routing.targetEdge}`;
                if (!edgeGroups.has(targetKey)) {
                    edgeGroups.set(targetKey, []);
                }
                edgeGroups.get(targetKey).push({ link, isSource: false });
                return; // Skip adding source to edge group
            }

            const sourceKey = `${link.source}:${link.routing.sourceEdge}`;
            const targetKey = `${link.target}:${link.routing.targetEdge}`;

            if (!edgeGroups.has(sourceKey)) {
                edgeGroups.set(sourceKey, []);
            }
            edgeGroups.get(sourceKey).push({ link, isSource: true });

            if (!edgeGroups.has(targetKey)) {
                edgeGroups.set(targetKey, []);
            }
            edgeGroups.get(targetKey).push({ link, isSource: false });
        });

        // For each edge group, distribute snap points evenly
        edgeGroups.forEach((group, key) => {
            const [nodeId, edge] = key.split(':');
            const node = nodes.find(n => n.id === nodeId);
            if (!node) {
                console.error(`[CSP ERROR] Node ${nodeId} not found!`);
                return;
            }

            const cx = node.x || 0;
            const cy = node.y || 0;
            const { halfWidth, halfHeight } = TransitionLayoutOptimizer.getNodeSize(node);
            console.log(`[CSP DEBUG] ${nodeId}.${edge}: center=(${cx.toFixed(1)}, ${cy.toFixed(1)}), type=${node.type}, size=${halfWidth}x${halfHeight}`);

            // Separate incoming and outgoing, then sort each by other node position
            const incomingGroup = group.filter(item => !item.isSource);
            const outgoingGroup = group.filter(item => item.isSource);

            const sortByOtherNodePosition = (a, b) => {
                const aNode = nodes.find(n => n.id === (a.isSource ? a.link.target : a.link.source));
                const bNode = nodes.find(n => n.id === (b.isSource ? b.link.target : b.link.source));

                if (edge === 'top' || edge === 'bottom') {
                    // Primary: Sort by horizontal position (x)
                    const xDiff = (aNode.x || 0) - (bNode.x || 0);
                    if (Math.abs(xDiff) > 0.1) {
                        return xDiff;
                    }

                    // Secondary: When x positions are same, sort by vertical position (y)
                    // For INCOMING links to horizontal edge: reverse y order to prevent crossings
                    // Higher sources (smaller y) should connect to rightmost targets (larger x)
                    if (!a.isSource && !b.isSource) {
                        return (bNode.y || 0) - (aNode.y || 0); // Reverse y order
                    }
                    return (aNode.y || 0) - (bNode.y || 0); // Normal y order for outgoing
                } else {
                    // Primary: Sort by vertical position (y)
                    const yDiff = (aNode.y || 0) - (bNode.y || 0);
                    if (Math.abs(yDiff) > 0.1) {
                        return yDiff;
                    }

                    // Secondary: When y positions are same, sort by horizontal position (x)
                    // For INCOMING links to vertical edge: reverse x order to prevent crossings
                    if (!a.isSource && !b.isSource) {
                        return (bNode.x || 0) - (aNode.x || 0); // Reverse x order
                    }
                    return (aNode.x || 0) - (bNode.x || 0); // Normal x order for outgoing
                }
            };

            incomingGroup.sort(sortByOtherNodePosition);
            outgoingGroup.sort(sortByOtherNodePosition);

            // Combine: incoming first, then outgoing
            const sortedGroup = [...incomingGroup, ...outgoingGroup];
            const count = sortedGroup.length;

            // Calculate distributed positions
            sortedGroup.forEach((item, index) => {
                const position = (index + 1) / (count + 1);
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

                // Assign calculated position
                const point = { x, y };
                if (item.isSource) {
                    if (item.link.routing) {
                        item.link.routing.sourcePoint = point;
                    }
                } else {
                    if (item.link.routing) {
                        item.link.routing.targetPoint = point;
                    }
                }

                const direction = item.isSource ? 'source' : 'target';
                console.log(`[OPTIMIZE CSP] ${item.link.source}→${item.link.target} ${direction} on ${nodeId}.${edge}: position ${index + 1}/${count} at (${x.toFixed(1)}, ${y.toFixed(1)})`);
            });
        });
    }
}
