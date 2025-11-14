// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Snap Calculator - Handles snap point prediction and calculation
 */

class SnapCalculator {
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

    hasInitialTransitionOnEdge(nodeId, edge) {
        return this.optimizer.links.some(link => {
            if (link.linkType !== 'initial') return false;
            if (this.getVisualTarget(link) !== nodeId) return false;

            // **PRIORITY: Use actual routing if available, fallback to prediction**
            if (link.routing && link.routing.targetEdge) {
                return link.routing.targetEdge === edge;
            }

            // Fallback: Predict edge based on node positions
            const source = this.optimizer.nodes.find(n => n.id === this.getVisualSource(link));
            const target = this.optimizer.nodes.find(n => n.id === this.getVisualTarget(link));
            if (!source || !target) return false;

            const targetEdge = this.optimizer.predictEdge(source, target, false);
            return targetEdge === edge;
        });
    }

    countConnectionsOnEdge(nodeId, edge) {
        const node = this.optimizer.nodes.find(n => n.id === nodeId);
        if (!node) return [];

        const connections = [];

        this.optimizer.links.forEach(link => {
            const source = this.optimizer.nodes.find(n => n.id === this.getVisualSource(link));
            const target = this.optimizer.nodes.find(n => n.id === this.getVisualTarget(link));
            if (!source || !target) return;

            // **TWO-PASS: Use confirmed directions if available, otherwise predict**
            let sourceEdge, targetEdge;

            if (link.routing) {
                // Use routing edges
                sourceEdge = link.routing.sourceEdge;
                targetEdge = link.routing.targetEdge;
            } else {
                // Fallback to prediction (ELK routing)
                sourceEdge = this.optimizer.predictEdge(source, target, true);
                targetEdge = this.optimizer.predictEdge(source, target, false);
            }

            // Check if this link uses the specified edge
            if (this.getVisualSource(link) === nodeId) {
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
            } else if (this.getVisualTarget(link) === nodeId) {
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

    estimateSourceSnapPosition(sourceId, targetId, link) {
        const source = this.optimizer.nodes.find(n => n.id === sourceId);
        const target = this.optimizer.nodes.find(n => n.id === targetId);
        if (!source || !target) return null;

        // **TWO-PASS: Use routing edges if available**
        let sourceEdge;
        if (link.routing) {
            // Use routing edges
            sourceEdge = link.routing.sourceEdge;
        } else {
            // Fallback to prediction
            sourceEdge = this.optimizer.predictEdge(source, target, true);
        }

        // Count ALL connections (incoming + outgoing) on that edge
        const connections = this.optimizer.countConnectionsOnEdge(sourceId, sourceEdge);

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

    calculateSnapPosition(nodeId, edge, linkId, direction) {
        let node = this.optimizer.nodes.find(n => n.id === nodeId);
        if (!node) return null;

        // If this node is a child of a collapsed parent, use the parent's coordinates instead
        const collapsedParent = this.optimizer.nodes.find(p => p.collapsed && p.children && p.children.includes(nodeId));
        if (collapsedParent) {
            console.log(`[SNAP] ${nodeId} is child of collapsed ${collapsedParent.id}, using parent coordinates`);
            node = collapsedParent;
            nodeId = collapsedParent.id;
        }

        const link = this.optimizer.links.find(l => l.id === linkId);
        if (!link) return null;

        // Special case: Initial pseudo-node transitions always use center position
        if (link.linkType === 'initial') {
            const cx = node.x || 0;
            const cy = node.y || 0;

            // Get size from node object (uses actual dimensions if available)
            const { halfWidth, halfHeight } = TransitionLayoutOptimizer.getNodeSize(node);

            console.log(`[SNAP INITIAL] ${nodeId} ${edge}: ${this.getVisualSource(link)}→${this.getVisualTarget(link)} (INITIAL) type=${node.type}, collapsed=${node.collapsed}, size=${halfWidth}x${halfHeight}`);

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
        if (this.optimizer.hasInitialTransitionOnEdge(nodeId, edge)) {
            console.log(`[SNAP BLOCKED] ${nodeId} ${edge}: ${this.getVisualSource(link)}→${this.getVisualTarget(link)} blocked - initial transition owns this edge`);
            return null;  // Force fallback to different edge or center
        }

        // Get all connections on this edge
        const allConnections = this.optimizer.countConnectionsOnEdge(nodeId, edge);

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
                aPos = this.optimizer.estimateSourceSnapPosition(a.link.source, a.link.target, a.link);
                if (aPos === null) {
                    aPos = isHorizontal ? (a.otherNode.x || 0) : (a.otherNode.y || 0);
                }
            }

            if (b.isSource) {
                bPos = isHorizontal ? (b.otherNode.x || 0) : (b.otherNode.y || 0);
            } else {
                bPos = this.optimizer.estimateSourceSnapPosition(b.link.source, b.link.target, b.link);
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
        const isIncoming = (this.getVisualTarget(link) === nodeId);
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

        console.log(`[SNAP] ${nodeId} ${edge}: ${this.getVisualSource(link)}→${this.getVisualTarget(link)} (${isIncoming ? 'IN' : 'OUT'}) at ${groupIndex + 1}/${group.length} in group, absolute ${absoluteIndex + 1}/${totalCount}, position ${position.toFixed(3)}`);

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

    getAllPossibleSnapCombinations(link, sourceNode, targetNode, reverseRouting = null) {
        const combinations = [];
        const edges = ['top', 'bottom', 'left', 'right'];

        edges.forEach(sourceEdge => {
            // Skip if source edge has initial transition blocking
            if (link.linkType !== 'initial' &&
                this.optimizer.hasInitialTransitionOnEdge(sourceNode.id, sourceEdge)) {
                return;
            }

            edges.forEach(targetEdge => {
                // Skip if target edge has initial transition blocking
                if (link.linkType !== 'initial' &&
                    this.optimizer.hasInitialTransitionOnEdge(targetNode.id, targetEdge)) {
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
                const sourcePoint = this.optimizer.getEdgeCenterPoint(sourceNode, sourceEdge);
                const targetPoint = this.optimizer.getEdgeCenterPoint(targetNode, targetEdge);

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

    calculateComboScore(link, combo, existingAssignment, allLinks) {
        // Calculate intersections with existing assignments
        let intersections = 0;
        for (const [assignedLinkId, assignedData] of existingAssignment) {
            intersections += this.optimizer.calculatePathIntersections(combo, assignedData.combo);
        }

        // Calculate node collisions
        let nodeCollisions = 0;
        const sourceNode = this.optimizer.nodes.find(n => n.id === this.getVisualSource(link));
        const targetNode = this.optimizer.nodes.find(n => n.id === this.getVisualTarget(link));

        if (sourceNode && this.optimizer.pathIntersectsNode(combo, sourceNode, { skipFirstSegment: true })) {
            nodeCollisions++;
        }
        if (targetNode && this.optimizer.pathIntersectsNode(combo, targetNode, { skipLastSegment: true })) {
            nodeCollisions++;
        }

        this.optimizer.nodes.forEach(node => {
            if (node.id === this.getVisualSource(link) || node.id === this.getVisualTarget(link)) return;
            if (this.optimizer.pathIntersectsNode(combo, node)) {
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

    distributeSnapPointsOnEdges(links, nodes) {
        console.log(`[DISTRIBUTE SNAP] Processing ${links.length} links, ${nodes.length} nodes`);
        console.log(`[DISTRIBUTE SNAP] Node IDs: ${nodes.map(n => n.id).join(', ')}`);
        // Group links by node and edge (combine incoming and outgoing)
        // Note: Expects already filtered links (transition and initial only)
        const edgeGroups = new Map(); // Key: "nodeId:edge", Value: [links]

        links.forEach(link => {
            // Check routing
            if (!link.routing) return;

            // **SPECIAL: Initial transitions start from center, not edge**
            if (link.linkType === 'initial') {
                const sourceNode = nodes.find(n => n.id === this.getVisualSource(link));
                if (sourceNode && sourceNode.type === 'initial-pseudo') {
                    // Use center of initial pseudo-node
                    const centerPoint = {
                        x: sourceNode.x || 0,
                        y: sourceNode.y || 0
                    };

                    if (link.routing) {
                        link.routing.sourcePoint = centerPoint;
                    }
                    console.log(`[OPTIMIZE CSP] ${this.getVisualSource(link)}→${this.getVisualTarget(link)} source at initial center: (${centerPoint.x.toFixed(1)}, ${centerPoint.y.toFixed(1)})`);
                }

                // Still need to add target to edge group
                const targetKey = `${this.getVisualTarget(link)}:${link.routing.targetEdge}`;
                if (!edgeGroups.has(targetKey)) {
                    edgeGroups.set(targetKey, []);
                }
                edgeGroups.get(targetKey).push({ link, isSource: false });
                return; // Skip adding source to edge group
            }

            const sourceKey = `${this.getVisualSource(link)}:${link.routing.sourceEdge}`;
            const targetKey = `${this.getVisualTarget(link)}:${link.routing.targetEdge}`;

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
            let [nodeId, edge] = key.split(':');
            let node = nodes.find(n => n.id === nodeId);
            if (!node) {
                console.error(`[CSP ERROR] Node ${nodeId} not found!`);
                return;
            }

            // If this node is a child of a collapsed parent, use the parent's coordinates instead
            // This handles visual redirect where hidden nodes (p) should use their collapsed parent (s2)
            const collapsedParent = nodes.find(p => p.collapsed && p.children && p.children.includes(nodeId));
            if (collapsedParent) {
                console.log(`[CSP] ${nodeId} is child of collapsed ${collapsedParent.id}, using parent coordinates`);
                node = collapsedParent;
                nodeId = collapsedParent.id;
            }

            const cx = node.x || 0;
            const cy = node.y || 0;
            const { halfWidth, halfHeight } = TransitionLayoutOptimizer.getNodeSize(node);
            console.log(`[CSP DEBUG] ${nodeId}.${edge}: center=(${cx.toFixed(1)}, ${cy.toFixed(1)}), type=${node.type}, collapsed=${node.collapsed}, size=${halfWidth}x${halfHeight}`);

            // Separate incoming and outgoing, then sort each by other node position
            const incomingGroup = group.filter(item => !item.isSource);
            const outgoingGroup = group.filter(item => item.isSource);

            const sortByOtherNodePosition = (a, b) => {
                const aNode = nodes.find(n => n.id === (a.isSource ? this.getVisualTarget(a.link) : this.getVisualSource(a.link)));
                const bNode = nodes.find(n => n.id === (b.isSource ? this.getVisualTarget(b.link) : this.getVisualSource(b.link)));

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
                console.log(`[OPTIMIZE CSP] ${this.getVisualSource(item.link)}→${this.getVisualTarget(item.link)} ${direction} on ${nodeId}.${edge}: position ${index + 1}/${count} at (${x.toFixed(1)}, ${y.toFixed(1)})`);
            });
        });
    }
}
