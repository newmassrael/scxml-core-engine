// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * TransitionLayoutOptimizer
 *
 * Optimizes transition arrow layout for minimal crossing and optimal snap positioning.
 * Handles connection analysis, snap point calculation, and crossing minimization.
 */
class TransitionLayoutOptimizer {
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
                // Horizontal alignment: use left/right
                return tx > sx ? 'right' : 'left';
            } else if (dx < 30) {
                // Vertical alignment: use top/bottom
                return sy < ty ? 'bottom' : 'top';
            } else {
                // Z-path: first segment is vertical
                return sy < ty ? 'bottom' : 'top';
            }
        } else {
            // Predict which edge the path enters from
            if (dy < 30) {
                // Horizontal alignment: use left/right
                return tx > sx ? 'left' : 'right';
            } else {
                // Vertical alignment or Z-path: last segment is vertical
                // sy > ty: source below target → path goes UP → enters target's BOTTOM
                // sy < ty: source above target → path goes DOWN → enters target's TOP
                return sy > ty ? 'bottom' : 'top';
            }
        }
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

            if (link.confirmedDirections) {
                // Use confirmed directions from Pass 1
                const outDir = link.confirmedDirections.outgoingDir;
                const inDir = link.confirmedDirections.incomingDir;

                // Map direction to edge
                if (outDir === 'to-top') sourceEdge = 'top';
                else if (outDir === 'to-bottom') sourceEdge = 'bottom';
                else if (outDir === 'to-left') sourceEdge = 'left';
                else if (outDir === 'to-right') sourceEdge = 'right';

                if (inDir === 'from-top') targetEdge = 'top';
                else if (inDir === 'from-bottom') targetEdge = 'bottom';
                else if (inDir === 'from-left') targetEdge = 'left';
                else if (inDir === 'from-right') targetEdge = 'right';
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

        // **TWO-PASS: Use confirmed direction if available**
        let sourceEdge;
        if (link.confirmedDirections) {
            const outDir = link.confirmedDirections.outgoingDir;
            if (outDir === 'to-top') sourceEdge = 'top';
            else if (outDir === 'to-bottom') sourceEdge = 'bottom';
            else if (outDir === 'to-left') sourceEdge = 'left';
            else if (outDir === 'to-right') sourceEdge = 'right';
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
                const halfWidth = 30;
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
                const halfHeight = 20;
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
        const halfWidth = 30;
        const halfHeight = 20;

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
}
