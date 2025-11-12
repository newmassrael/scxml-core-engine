// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * TransitionLayoutOptimizer
 *
 * Optimizes transition arrow layout for minimal crossing and optimal snap positioning.
 * Handles connection analysis, snap point calculation, and crossing minimization.
 */
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
            if (skipLastSegment && i === segments.length - 1) continue;

            // Check if segment intersects with node bounding box
            if (this.segmentIntersectsRect(segment, nodeLeft, nodeTop, nodeRight, nodeBottom)) {
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
     * Optimize snap point assignments for all links
     * Uses Constraint Satisfaction Problem (CSP) solver for globally optimal routing
     * Phase 1: CSP solver assigns optimal edges (hard constraints + soft constraints)
     * Phase 2: Distribute snap points on each edge
     * @param {Array} links - Array of link objects
     * @param {Array} nodes - Array of node objects
     */
    optimizeSnapPointAssignments(links, nodes, useGreedy = false) {
        // Adaptive Algorithm Selection:
        // - useGreedy=true: Fast greedy algorithm for real-time drag (1-5ms)
        // - useGreedy=false: Optimal CSP solver for final result (50-200ms)
        
        if (useGreedy) {
            console.log('[OPTIMIZE] Using fast greedy algorithm (drag mode)...');
            this.optimizeSnapPointAssignmentsGreedy(links, nodes);
            return;
        }

        console.log('[OPTIMIZE] Using Constraint Satisfaction Solver (optimal mode)...');

        // Check if ConstraintSolver is available
        if (typeof ConstraintSolver === 'undefined') {
            console.warn('[OPTIMIZE] ConstraintSolver not found, falling back to greedy algorithm');
            this.optimizeSnapPointAssignmentsGreedy(links, nodes);
            return;
        }

        // Create CSP solver
        const solver = new ConstraintSolver(links, nodes, this);

        // Solve
        const solution = solver.solve();

        if (!solution) {
            console.error('[OPTIMIZE] CSP failed, falling back to greedy algorithm');
            this.optimizeSnapPointAssignmentsGreedy(links, nodes);
            return;
        }

        // Apply solution to links
        solution.forEach((assignment, linkId) => {
            const link = links.find(l => l.id === linkId);
            if (link) {
                link.routing = RoutingState.fromEdges(
                    assignment.sourceEdge,
                    assignment.targetEdge
                );

                console.log(`[OPTIMIZE CSP] ${link.source}→${link.target}: ${assignment.sourceEdge}→${assignment.targetEdge}`);
            }
        });

        // Phase 2: Distribute snap points on each edge
        const sortedLinks = [...links].sort((a, b) => {
            if (a.linkType === 'initial' && b.linkType !== 'initial') return -1;
            if (a.linkType !== 'initial' && b.linkType === 'initial') return 1;
            return 0;
        });
        this.distributeSnapPointsOnEdges(sortedLinks, nodes);

        // console.log(`Optimized snap points for ${links.length} links using CSP`);
    }

    /**
     * Fallback greedy algorithm (original Phase 1 + Phase 1.5)
     * Used when CSP solver is not available or fails
     */
    optimizeSnapPointAssignmentsGreedy(links, nodes) {
        console.log('[OPTIMIZE GREEDY] Using greedy algorithm (fallback)...');

        const assignedPaths = [];

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

        // Phase 1: Assign optimal edges to each link
        sortedLinks.forEach(link => {
            const sourceNode = nodes.find(n => n.id === link.source);
            const targetNode = nodes.find(n => n.id === link.target);

            if (!sourceNode || !targetNode) return;

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

                // Score with original penalty weights
                const score = tooCloseSnap * 100000 + nodeCollisions * 10000 + intersections * 1000 + combo.distance;

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

        // Phase 2: Distribute snap points on each edge
        this.distributeSnapPointsOnEdges(sortedLinks, nodes);

        console.log(`Optimized snap points for ${links.length} links using greedy algorithm`);
    }

    /**
     * Distribute snap points evenly on each edge
     * @param {Array} links - Array of link objects
     * @param {Array} nodes - Array of node objects
     */
    distributeSnapPointsOnEdges(links, nodes) {
        // Group links by node and edge (combine incoming and outgoing)
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
                    console.log(`[OPTIMIZE PHASE 2] ${link.source}→${link.target} source at initial center: (${centerPoint.x.toFixed(1)}, ${centerPoint.y.toFixed(1)})`);
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
                console.error(`[PHASE 2 ERROR] Node ${nodeId} not found!`);
                return;
            }

            const cx = node.x || 0;
            const cy = node.y || 0;
            const { halfWidth, halfHeight } = TransitionLayoutOptimizer.getNodeSize(node);
            console.log(`[PHASE 2 DEBUG] ${nodeId}.${edge}: center=(${cx.toFixed(1)}, ${cy.toFixed(1)}), type=${node.type}, size=${halfWidth}x${halfHeight}`);

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
                console.log(`[OPTIMIZE PHASE 2] ${item.link.source}→${item.link.target} ${direction} on ${nodeId}.${edge}: position ${index + 1}/${count} at (${x.toFixed(1)}, ${y.toFixed(1)})`);
            });
        });
    }
}
