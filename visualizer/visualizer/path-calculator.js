// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Path Calculator - Handles path computation, collision detection, boundary points
 */

class PathCalculator {
    constructor(visualizer) {
        this.visualizer = visualizer;

        // Label positioning constants
        this.COORDINATE_TOLERANCE = 1.0;      // Pixel tolerance for straight line detection
        this.LABEL_OFFSET_VERTICAL = 8;       // Vertical offset for bent paths
        this.LABEL_OFFSET_HORIZONTAL = 10;    // Horizontal offset for bent paths
        this.MIN_LABEL_DISTANCE = 20;         // Minimum distance from state edges
    }

    getTransitionLabelText(transition) {
        // Generate hierarchical HTML structure for better readability
        const parts = [];

        // Eventless indicator - always show first if transition is eventless (W3C SCXML 3.12)
        if (transition.eventless === true) {
            parts.push(`<div class="label-eventless">⚡ eventless</div>`);
        }
        // Event name (W3C SCXML 3.12.1) - only for event-based transitions
        else if (transition.event) {
            parts.push(`<div class="label-event">${transition.event}</div>`);
        }

        // Condition (guard) - W3C SCXML 3.12.1
        if (transition.cond) {
            const icon = '🔍';  // Condition icon
            parts.push(`<div class="label-condition">${icon} [${transition.cond}]</div>`);
        }

        // Actions - W3C SCXML 3.7 (using ActionFormatter for consistency)
        if (transition.actions && transition.actions.length > 0) {
            transition.actions.forEach(action => {
                // Use ActionFormatter for consistent formatting
                const formatted = (typeof ActionFormatter !== 'undefined')
                    ? ActionFormatter.formatAction(action)
                    : { main: 'action', details: [] };

                // Wrap in label-action div with arrow prefix
                parts.push(`<div class="label-action">↳ ${formatted.main}</div>`);
            });
        }

        // Fallback for completely empty transitions (no event, no eventless flag, no condition, no actions)
        if (parts.length === 0) {
            parts.push(`<div class="label-always">(always)</div>`);
        }

        // Build label classes with color variant
        let labelClasses = 'transition-label';
        if (transition.colorIndex !== null && transition.colorIndex !== undefined) {
            labelClasses += ` label-color-${transition.colorIndex}`;
        }

        // Add data-transition-id attribute
        const transitionId = transition.transitionId ? `data-transition-id="${transition.transitionId}"` : '';

        return `<div class="${labelClasses}" ${transitionId}>${parts.join('')}</div>`;
    }

    /**
     * Helper: Check if two coordinates are close enough to be considered a straight line
     * @param {number} coord1 - First coordinate
     * @param {number} coord2 - Second coordinate
     * @returns {boolean} True if coordinates are within tolerance
     */
    _isStraightPath(coord1, coord2) {
        return Math.abs(coord1 - coord2) < this.COORDINATE_TOLERANCE;
    }

    /**
     * Helper: Calculate midpoint between two values with optional offset
     * @param {number} start - Start value
     * @param {number} end - End value
     * @param {number} offset - Optional offset to apply (default: 0)
     * @returns {number} Calculated midpoint
     */
    _calculateMidpoint(start, end, offset = 0) {
        if (start == null || end == null) {
            console.error('[MIDPOINT] Invalid coordinates:', { start, end });
            return 0;
        }
        return (start + end) / 2 + offset;
    }

    /**
     * Helper: Create label position object with logging
     * @param {number} x - X coordinate
     * @param {number} y - Y coordinate
     * @param {string} pathType - Description of path type for logging
     * @param {string} transitionId - Transition identifier
     * @returns {{x: number, y: number}} Position object
     */
    _createLabelPosition(x, y, pathType, transitionId) {
        const pos = { x, y };
        console.log(`[LABEL POS] ${transitionId}: ${pathType} → position(${x.toFixed(1)}, ${y.toFixed(1)})`);
        return pos;
    }

    /**
     * Helper: Ensure minimum distance from state edge
     * @param {number} position - Calculated position
     * @param {number} stateEdge - State edge position
     * @returns {number} Adjusted position
     */
    _ensureMinimumDistance(position, stateEdge) {
        const distance = Math.abs(position - stateEdge);
        if (distance < this.MIN_LABEL_DISTANCE) {
            const direction = position > stateEdge ? 1 : -1;
            return stateEdge + (this.MIN_LABEL_DISTANCE * direction);
        }
        return position;
    }

    getTransitionLabelPosition(transition) {
        const transitionId = `${transition.source}→${transition.target}`;
        console.log(`[LABEL POS] Calculating position for ${transitionId}`);

        // Use routing information to get actual path coordinates
        if (transition.routing && transition.routing.sourcePoint && transition.routing.targetPoint) {
            const start = transition.routing.sourcePoint;
            const end = transition.routing.targetPoint;
            const sourceEdge = transition.routing.sourceEdge;
            const targetEdge = transition.routing.targetEdge;

            const sx = start.x;
            const sy = start.y;
            const tx = end.x;
            const ty = end.y;

            console.log(`[LABEL POS] ${transitionId}: Has routing - source(${sx}, ${sy}) [${sourceEdge}] → target(${tx}, ${ty}) [${targetEdge}]`);

            const MIN_SEGMENT = PATH_CONSTANTS.MIN_SEGMENT_LENGTH;
            const sourceIsVertical = (sourceEdge === 'top' || sourceEdge === 'bottom');
            const targetIsVertical = (targetEdge === 'top' || targetEdge === 'bottom');

            // Calculate the middle segment of the path based on edge types
            if (sourceIsVertical && targetIsVertical) {
                // Both vertical edges: VHV path (vertical-horizontal-vertical)

                if (this._isStraightPath(sx, tx)) {
                    // Straight vertical transition: place label at visual midpoint
                    const midY = this._calculateMidpoint(sy, ty);
                    const adjustedY = this._ensureMinimumDistance(midY, sy);
                    return this._createLabelPosition(sx, adjustedY, 'Straight vertical', transitionId);
                } else {
                    // Bent VHV path: place label on horizontal segment
                    const cornerY = sourceEdge === 'top'
                        ? sy - MIN_SEGMENT
                        : sy + MIN_SEGMENT;
                    const labelX = this._calculateMidpoint(sx, tx);
                    const rawLabelY = cornerY - this.LABEL_OFFSET_VERTICAL;
                    const labelY = this._ensureMinimumDistance(rawLabelY, sy);
                    return this._createLabelPosition(labelX, labelY, 'Bent VHV path', transitionId);
                }
            } else if (!sourceIsVertical && !targetIsVertical) {
                // Both horizontal edges: HVH path (horizontal-vertical-horizontal)

                if (this._isStraightPath(sy, ty)) {
                    // Straight horizontal transition: place label at visual midpoint
                    const midX = this._calculateMidpoint(sx, tx);
                    const adjustedX = this._ensureMinimumDistance(midX, sx);
                    return this._createLabelPosition(adjustedX, sy, 'Straight horizontal', transitionId);
                } else {
                    // Bent HVH path: place label on vertical segment
                    const cornerX = sourceEdge === 'right'
                        ? sx + MIN_SEGMENT
                        : sx - MIN_SEGMENT;
                    const rawLabelX = cornerX + this.LABEL_OFFSET_HORIZONTAL;
                    const labelX = this._ensureMinimumDistance(rawLabelX, sx);
                    const labelY = this._calculateMidpoint(sy, ty);
                    return this._createLabelPosition(labelX, labelY, 'Bent HVH path', transitionId);
                }
            } else if (sourceIsVertical && !targetIsVertical) {
                // Source vertical, target horizontal: V→H path
                const cornerY = sourceEdge === 'top'
                    ? sy - MIN_SEGMENT
                    : sy + MIN_SEGMENT;
                const labelX = this._calculateMidpoint(sx, tx);
                const rawLabelY = cornerY - this.LABEL_OFFSET_VERTICAL;
                const labelY = this._ensureMinimumDistance(rawLabelY, sy);
                return this._createLabelPosition(labelX, labelY, 'V→H path', transitionId);
            } else {
                // Source horizontal, target vertical: H→V path
                const cornerX = sourceEdge === 'right'
                    ? sx + MIN_SEGMENT
                    : sx - MIN_SEGMENT;
                const rawLabelX = cornerX + this.LABEL_OFFSET_HORIZONTAL;
                const labelX = this._ensureMinimumDistance(rawLabelX, sx);
                const labelY = this._calculateMidpoint(sy, ty);
                return this._createLabelPosition(labelX, labelY, 'H→V path', transitionId);
            }
        }

        // Fallback to simple midpoint (no routing information)
        console.log(`[LABEL POS] ${transitionId}: No routing info - using fallback midpoint`);
        const sourceNode = this.visualizer.nodes.find(n => n.id === transition.source);
        const targetNode = this.visualizer.nodes.find(n => n.id === transition.target);

        if (!sourceNode || !targetNode) {
            console.log(`[LABEL POS] ${transitionId}: ERROR - Source or target node not found`);
            return { x: 0, y: 0 };
        }

        const labelX = this._calculateMidpoint(sourceNode.x, targetNode.x);
        const labelY = this._calculateMidpoint(sourceNode.y, targetNode.y, -this.LABEL_OFFSET_VERTICAL);
        return this._createLabelPosition(labelX, labelY, 'Fallback midpoint', transitionId);
    }

    getNodeSide(node, toX, toY) {
        const cx = node.x || 0;
        const cy = node.y || 0;
        const dx = toX - cx;
        const dy = toY - cy;

        // Use angle to determine side
        const angle = Math.atan2(dy, dx) * 180 / Math.PI;

        // -45 to 45: right, 45 to 135: bottom, 135 to 180 or -180 to -135: left, -135 to -45: top
        if (angle >= -45 && angle < 45) return 'right';
        if (angle >= 45 && angle < 135) return 'bottom';
        if (angle >= 135 || angle < -135) return 'left';
        return 'top';
    }

    getNodeBoundaryPoint(node, fromX, fromY, link, isSource, connections) {
        const cx = node.x || 0;
        const cy = node.y || 0;

        // Direction vector from node center to target
        const dx = fromX - cx;
        const dy = fromY - cy;
        const distance = Math.sqrt(dx * dx + dy * dy);

        if (distance === 0) return { x: cx, y: cy };

        // Normalize direction
        const ndx = dx / distance;
        const ndy = dy / distance;

        // Get node dimensions based on type
        if (node.type === 'initial-pseudo') {
            // Circle with radius 10
            return {
                x: cx + ndx * 10,
                y: cy + ndy * 10
            };
        } else if (node.type === 'history') {
            // Circle with radius 20
            return {
                x: cx + ndx * 20,
                y: cy + ndy * 20
            };
        } else if (node.type === 'atomic' || node.type === 'final') {
            // Rectangle with smart snapping (use actual node dimensions)
            const halfWidth = (node.width || 60) / 2;
            const halfHeight = (node.height || 40) / 2;

            // Determine which side this connection is on
            const side = this.visualizer.getNodeSide(node, fromX, fromY);

            // Get snap position on this side
            let snapX = cx, snapY = cy;

            if (connections && connections[node.id]) {
                const sideConnections = connections[node.id][side];
                const index = sideConnections.findIndex(c =>
                    c.link.id === link.id && c.isSource === isSource
                );

                if (index >= 0 && sideConnections.length > 0) {
                    const count = sideConnections.length;
                    const position = (index + 1) / (count + 1); // Divide side into (count+1) segments

                    if (side === 'top') {
                        snapX = cx - halfWidth + (halfWidth * 2 * position);
                        snapY = cy - halfHeight;
                    } else if (side === 'bottom') {
                        snapX = cx - halfWidth + (halfWidth * 2 * position);
                        snapY = cy + halfHeight;
                    } else if (side === 'left') {
                        snapX = cx - halfWidth;
                        snapY = cy - halfHeight + (halfHeight * 2 * position);
                    } else if (side === 'right') {
                        snapX = cx + halfWidth;
                        snapY = cy - halfHeight + (halfHeight * 2 * position);
                    }

                    // Snap point is exactly on boundary
                    return { x: snapX, y: snapY };
                }
            }

            // Fallback: no snapping, use angle-based intersection
            const tx = Math.abs(ndx) > 0 ? halfWidth / Math.abs(ndx) : Infinity;
            const ty = Math.abs(ndy) > 0 ? halfHeight / Math.abs(ndy) : Infinity;
            const t = Math.min(tx, ty);

            return {
                x: cx + ndx * t,
                y: cy + ndy * t
            };
        } else if (SCXMLVisualizer.isCompoundOrParallel(node)) {
            // Use node's width and height
            const halfWidth = (node.width || 60) / 2;
            const halfHeight = (node.height || 40) / 2;

            const tx = Math.abs(ndx) > 0 ? halfWidth / Math.abs(ndx) : Infinity;
            const ty = Math.abs(ndy) > 0 ? halfHeight / Math.abs(ndy) : Infinity;
            const t = Math.min(tx, ty);

            return {
                x: cx + ndx * t,
                y: cy + ndy * t
            };
        }

        // Default: return center
        return { x: cx, y: cy };
    }

    getOrthogonalIncomingDirection(start, end) {
        const dx = Math.abs(end.x - start.x);
        const dy = Math.abs(end.y - start.y);

        // Already aligned - direct line
        if (dx < 1) {
            // Vertical line
            return end.y > start.y ? 'from-top' : 'from-bottom';
        }
        if (dy < 1) {
            // Horizontal line
            return end.x > start.x ? 'from-left' : 'from-right';
        }

        // Z-shaped path with midpoint
        const midY = (start.y + end.y) / 2;

        // Last segment is vertical: (end.x, midY) → (end.x, end.y)
        return end.y > midY ? 'from-top' : 'from-bottom';
    }

    getOrthogonalOutgoingDirection(start, end) {
        const dx = Math.abs(end.x - start.x);
        const dy = Math.abs(end.y - start.y);

        // Already aligned - direct line
        if (dx < 1) {
            // Vertical line
            return end.y > start.y ? 'to-bottom' : 'to-top';
        }
        if (dy < 1) {
            // Horizontal line
            return end.x > start.x ? 'to-right' : 'to-left';
        }

        // Z-shaped path with midpoint
        const midY = (start.y + end.y) / 2;

        // First segment is vertical: (start.x, start.y) → (start.x, midY)
        return midY > start.y ? 'to-bottom' : 'to-top';
    }

    getOrthogonalBoundaryPoint(node, direction, link = null, isSource = true, connections = null) {
        const cx = node.x || 0;
        const cy = node.y || 0;

        if (node.type === 'atomic' || node.type === 'final') {
            const halfWidth = PATH_CONSTANTS.INITIAL_NODE_HALF_WIDTH;
            const halfHeight = 20;

            // **PRIORITY: Use routing if available (don't let direction override it)**
            if (link && link.routing) {
                const optPoint = isSource ? link.routing.sourcePoint : link.routing.targetPoint;

                if (optPoint) {
                    // Use the optimized snap point directly, no fallback needed
                    return { x: optPoint.x, y: optPoint.y };
                }
            }

            // Map direction to side
            let side = null;
            if (direction === 'from-top' || direction === 'to-top') {
                side = 'top';
            } else if (direction === 'from-bottom' || direction === 'to-bottom') {
                side = 'bottom';
            } else if (direction === 'from-left' || direction === 'to-left') {
                side = 'left';
            } else if (direction === 'from-right' || direction === 'to-right') {
                side = 'right';
            }

            // Smart snapping: use layout optimizer to calculate optimal snap position
            if (side && link) {
                const snapResult = this.visualizer.layoutOptimizer.calculateSnapPosition(
                    node.id,
                    side,
                    link.id,
                    direction
                );

                if (snapResult) {
                    return { x: snapResult.x, y: snapResult.y };
                }

                // If blocked by initial transition, try alternative edges
                if (this.visualizer.layoutOptimizer.hasInitialTransitionOnEdge(node.id, side)) {
                    console.log(`[FALLBACK] ${node.id} ${side}: blocked, trying alternative edge`);

                    // Try alternative edges based on original direction
                    const alternatives = [];
                    if (side === 'top' || side === 'bottom') {
                        alternatives.push('left', 'right', side === 'top' ? 'bottom' : 'top');
                    } else {
                        alternatives.push('top', 'bottom', side === 'left' ? 'right' : 'left');
                    }

                    // Try each alternative
                    for (const altSide of alternatives) {
                        if (!this.visualizer.layoutOptimizer.hasInitialTransitionOnEdge(node.id, altSide)) {
                            console.log(`[FALLBACK] ${node.id}: using ${altSide} instead of ${side}`);

                            // Calculate proper snap position for alternative edge
                            const altDirection = isSource ? `to-${altSide}` : `from-${altSide}`;
                            const altSnapResult = this.visualizer.layoutOptimizer.calculateSnapPosition(
                                node.id,
                                altSide,
                                link.id,
                                altDirection
                            );

                            // **DO NOT MODIFY routing - it's read-only!**
                            // Return the alternative snap position without modifying routing

                            if (altSnapResult) {
                                return { x: altSnapResult.x, y: altSnapResult.y };
                            }

                            // Fallback to edge center if snap calculation fails
                            if (altSide === 'top') {
                                return { x: cx, y: cy - halfHeight };
                            } else if (altSide === 'bottom') {
                                return { x: cx, y: cy + halfHeight };
                            } else if (altSide === 'left') {
                                return { x: cx - halfWidth, y: cy };
                            } else if (altSide === 'right') {
                                return { x: cx + halfWidth, y: cy };
                            }
                        }
                    }
                }
            }

            // Fallback: no smart snapping, use edge center
            if (direction === 'from-top' || direction === 'to-top') {
                return { x: cx, y: cy - halfHeight };
            } else if (direction === 'from-bottom' || direction === 'to-bottom') {
                return { x: cx, y: cy + halfHeight };
            } else if (direction === 'from-left' || direction === 'to-left') {
                return { x: cx - halfWidth, y: cy };
            } else if (direction === 'from-right' || direction === 'to-right') {
                return { x: cx + halfWidth, y: cy };
            }
        } else if (node.type === 'initial-pseudo') {
            const radius = 10;
            if (direction === 'from-top' || direction === 'to-top') {
                return { x: cx, y: cy - radius };
            } else if (direction === 'from-bottom' || direction === 'to-bottom') {
                return { x: cx, y: cy + radius };
            } else if (direction === 'from-left' || direction === 'to-left') {
                return { x: cx - radius, y: cy };
            } else if (direction === 'from-right' || direction === 'to-right') {
                return { x: cx + radius, y: cy };
            }
        } else if (node.type === 'history') {
            const radius = 20;
            if (direction === 'from-top' || direction === 'to-top') {
                return { x: cx, y: cy - radius };
            } else if (direction === 'from-bottom' || direction === 'to-bottom') {
                return { x: cx, y: cy + radius };
            } else if (direction === 'from-left' || direction === 'to-left') {
                return { x: cx - radius, y: cy };
            } else if (direction === 'from-right' || direction === 'to-right') {
                return { x: cx + radius, y: cy };
            }
        }

        // Default: return center
        return { x: cx, y: cy };
    }

    getNodeBounds(node) {
        const cx = node.x || 0;
        const cy = node.y || 0;

        if (node.type === 'atomic' || node.type === 'final') {
            // Use actual node dimensions (not hardcoded 60x40)
            const halfWidth = (node.width || 60) / 2;
            const halfHeight = (node.height || 40) / 2;
            return {
                left: cx - halfWidth,
                right: cx + halfWidth,
                top: cy - halfHeight,
                bottom: cy + halfHeight
            };
        } else if (node.type === 'initial-pseudo') {
            return {
                left: cx - 10,
                right: cx + 10,
                top: cy - 10,
                bottom: cy + 10
            };
        } else if (node.type === 'history') {
            return {
                left: cx - 20,
                right: cx + 20,
                top: cy - 20,
                bottom: cy + 20
            };
        } else if (SCXMLVisualizer.isCompoundOrParallel(node)) {
            const halfWidth = (node.width || 60) / 2;
            const halfHeight = (node.height || 40) / 2;
            return {
                left: cx - halfWidth,
                right: cx + halfWidth,
                top: cy - halfHeight,
                bottom: cy + halfHeight
            };
        }

        return { left: cx, right: cx, top: cy, bottom: cy };
    }

    horizontalLineIntersectsNode(y, xStart, xEnd, node) {
        const bounds = this.visualizer.getNodeBounds(node);

        // Check if y is within node's vertical range
        if (y < bounds.top || y > bounds.bottom) {
            return false;
        }

        // Check if horizontal line segment overlaps with node's horizontal range
        const lineLeft = Math.min(xStart, xEnd);
        const lineRight = Math.max(xStart, xEnd);

        return !(lineRight < bounds.left || lineLeft > bounds.right);
    }

    verticalLineIntersectsNode(x, yStart, yEnd, node) {
        const bounds = this.visualizer.getNodeBounds(node);

        // Check if x is within node's horizontal bounds
        if (x < bounds.left || x > bounds.right) {
            return false;
        }

        // Check if vertical segment overlaps node's vertical bounds
        const lineTop = Math.min(yStart, yEnd);
        const lineBottom = Math.max(yStart, yEnd);

        return !(lineBottom < bounds.top || lineTop > bounds.bottom);
    }

    getObstacleNodes(sourceId, targetId) {
        return this.visualizer.nodes.filter(node => 
            node.id !== sourceId && 
            node.id !== targetId &&
            node.type !== 'initial'  // Initial state markers are small, ignore them
        );
    }

    findCollisionFreeY(sx, sy, tx, ty, obstacles) {
        const candidates = [];
        const margin = 15;

        // Strategy 1: Try midpoint
        const midY = (sy + ty) / 2;
        candidates.push(midY);

        // Strategy 2: Try routing above all obstacles
        const maxTop = Math.max(
            ...obstacles.map(node => this.visualizer.getNodeBounds(node).top),
            sy, ty
        );
        candidates.push(maxTop - margin);

        // Strategy 3: Try routing below all obstacles
        const minBottom = Math.min(
            ...obstacles.map(node => this.visualizer.getNodeBounds(node).bottom),
            sy, ty
        );
        candidates.push(minBottom + margin);

        // Strategy 4: Try routing at source height
        candidates.push(sy);

        // Strategy 5: Try routing at target height
        candidates.push(ty);

        // Test each candidate and find first that doesn't collide
        for (const candidateY of candidates) {
            let hasCollision = false;

            // Check horizontal segment collision
            for (const obstacle of obstacles) {
                if (this.visualizer.horizontalLineIntersectsNode(candidateY, sx, tx, obstacle)) {
                    hasCollision = true;
                    break;
                }
            }

            if (!hasCollision) {
                // Also verify vertical segments don't collide
                const verticalCollision = obstacles.some(obstacle =>
                    this.visualizer.verticalLineIntersectsNode(sx, sy, candidateY, obstacle) ||
                    this.visualizer.verticalLineIntersectsNode(tx, candidateY, ty, obstacle)
                );

                if (!verticalCollision) {
                    return candidateY;
                }
            }
        }

        // Fallback: return midpoint (better than nothing)
        return midY;
    }

    calculateLinkDirections(sourceNode, targetNode, link) {
        // **PRIORITY: If routing exists, calculate midY and store in routing**
        // Don't run collision avoidance again - optimizer already calculated optimal path
        if (link.routing && link.routing.sourceEdge && link.routing.targetEdge) {
            const sy = link.routing.sourcePoint.y;
            const ty = link.routing.targetPoint.y;
            const midY = (sy + ty) / 2;

            // Store midY in routing for z-path collision avoidance
            link.routing.midY = midY;
            return;
        }

        // **FALLBACK: routing should always exist after optimizer runs for transition/initial links**
        // Containment and delegation links don't have routing (hierarchical structure, not routing path)
        // Skip warning if either node lacks coordinates (expected when ELK doesn't layout nested hierarchies)
        const hasCoords = sourceNode && targetNode &&
                         sourceNode.x !== undefined && sourceNode.y !== undefined &&
                         targetNode.x !== undefined && targetNode.y !== undefined;

        if (link.linkType !== 'containment' && link.linkType !== 'delegation' && hasCoords) {
            console.warn(`[CALC DIR WARNING] ${link.source}→${link.target}: No routing found! Optimizer should have run first.`);
        }
    }

    createOrthogonalPath(sourceNode, targetNode, link, connections) {
        // **OPTIMIZED SNAP POINTS: Use routing if available**
        if (link.routing) {
            const start = link.routing.sourcePoint;
            const end = link.routing.targetPoint;
            const sourceEdge = link.routing.sourceEdge;
            const targetEdge = link.routing.targetEdge;

            const sx = start.x;
            const sy = start.y;
            const tx = end.x;
            const ty = end.y;

            // Debug mode: log path coordinates
            if (this.visualizer.debugMode) {
                console.log(`[PATH DEBUG] ${link.source}→${link.target}: source=(${sx.toFixed(1)}, ${sy.toFixed(1)}), target=(${tx.toFixed(1)}, ${ty.toFixed(1)})`);
            }

            const dx = Math.abs(tx - sx);
            const dy = Math.abs(ty - sy);

            // Check if direct line (horizontal or vertical alignment)
            if (dx < 1 || dy < 1) {
                // Direct line
                return `M ${sx} ${sy} L ${tx} ${ty}`;
            }

            // Create orthogonal path based on edge directions with minimum segment lengths
            const sourceIsVertical = (sourceEdge === 'top' || sourceEdge === 'bottom');
            const targetIsVertical = (targetEdge === 'top' || targetEdge === 'bottom');
            const MIN_SEGMENT = PATH_CONSTANTS.MIN_SEGMENT_LENGTH;  // Minimum horizontal/vertical segment length

            if (sourceIsVertical && targetIsVertical) {
                // Both vertical edges: vertical-horizontal-vertical (5 points)
                // Ensure minimum vertical segments from source and target
                let y1;
                if (sourceEdge === 'top') {
                    y1 = sy - MIN_SEGMENT;
                } else { // bottom
                    y1 = sy + MIN_SEGMENT;
                }

                let y2;
                if (targetEdge === 'top') {
                    y2 = ty - MIN_SEGMENT;
                } else { // bottom
                    y2 = ty + MIN_SEGMENT;
                }

                // Path: start → vertical MIN_SEGMENT → horizontal to target x → vertical MIN_SEGMENT → end
                return `M ${sx} ${sy} L ${sx} ${y1} L ${tx} ${y1} L ${tx} ${y2} L ${tx} ${ty}`;
            } else if (!sourceIsVertical && !targetIsVertical) {
                // Both horizontal edges: horizontal-vertical-horizontal (5 points)
                // Ensure minimum horizontal segments from source and target
                let x1;
                if (sourceEdge === 'right') {
                    x1 = sx + MIN_SEGMENT;
                } else { // left
                    x1 = sx - MIN_SEGMENT;
                }

                let x2;
                if (targetEdge === 'right') {
                    x2 = tx + MIN_SEGMENT;
                } else { // left
                    x2 = tx - MIN_SEGMENT;
                }

                // Path: start → horizontal MIN_SEGMENT → vertical to target y → horizontal MIN_SEGMENT → end
                const path = `M ${sx} ${sy} L ${x1} ${sy} L ${x1} ${ty} L ${x2} ${ty} L ${tx} ${ty}`;
                if (this.visualizer.debugMode) {
                    console.log(`[PATH GEN] ${link.source}→${link.target} ${sourceEdge}→${targetEdge}: M(${sx.toFixed(1)},${sy.toFixed(1)}) →H(${x1.toFixed(1)},${sy.toFixed(1)}) →V(${x1.toFixed(1)},${ty.toFixed(1)}) →H(${x2.toFixed(1)},${ty.toFixed(1)}) →H(${tx.toFixed(1)},${ty.toFixed(1)})`);
                }
                return path;
            } else if (sourceIsVertical && !targetIsVertical) {
                // Source vertical, target horizontal: vertical-then-horizontal (5 points)
                // Ensure minimum segments
                let y1;
                if (sourceEdge === 'top') {
                    y1 = sy - MIN_SEGMENT;
                } else { // bottom
                    y1 = sy + MIN_SEGMENT;
                }

                let x2;
                if (targetEdge === 'right') {
                    x2 = tx + MIN_SEGMENT;
                } else { // left
                    x2 = tx - MIN_SEGMENT;
                }

                // Path: start → vertical MIN_SEGMENT → horizontal to x2 → horizontal MIN_SEGMENT to end
                return `M ${sx} ${sy} L ${sx} ${y1} L ${x2} ${y1} L ${x2} ${ty} L ${tx} ${ty}`;
            } else {
                // Source horizontal, target vertical: horizontal-then-vertical (5 points)
                // Ensure minimum segments
                let x1;
                if (sourceEdge === 'right') {
                    x1 = sx + MIN_SEGMENT;
                } else { // left
                    x1 = sx - MIN_SEGMENT;
                }

                let y2;
                if (targetEdge === 'top') {
                    y2 = ty - MIN_SEGMENT;
                } else { // bottom
                    y2 = ty + MIN_SEGMENT;
                }

                // Path: start → horizontal MIN_SEGMENT → vertical to y2 → vertical MIN_SEGMENT to end
                const path = `M ${sx} ${sy} L ${x1} ${sy} L ${x1} ${y2} L ${tx} ${y2} L ${tx} ${ty}`;
                if (this.visualizer.debugMode) {
                    console.log(`[PATH GEN] ${link.source}→${link.target} ${sourceEdge}→${targetEdge}: M(${sx.toFixed(1)},${sy.toFixed(1)}) →H(${x1.toFixed(1)},${sy.toFixed(1)}) →V(${x1.toFixed(1)},${y2.toFixed(1)}) →H(${tx.toFixed(1)},${y2.toFixed(1)}) →V(${tx.toFixed(1)},${ty.toFixed(1)})`);
                }
                return path;
            }
        }

        // **FALLBACK: routing should always exist after optimizer runs for transition/initial links**
        // Containment and delegation links don't have routing (hierarchical structure, not routing path)
        // Skip warning if either node lacks coordinates (expected when ELK doesn't layout nested hierarchies)
        const hasCoords = sourceNode && targetNode &&
                         sourceNode.x !== undefined && sourceNode.y !== undefined &&
                         targetNode.x !== undefined && targetNode.y !== undefined;

        if (link.linkType !== 'containment' && link.linkType !== 'delegation' && hasCoords) {
            console.warn(`[PATH WARNING] ${link.source}→${link.target}: No routing found! Falling back to node centers.`);
        }

        // Draw direct line as emergency fallback
        const sx = sourceNode.x || 0;
        const sy = sourceNode.y || 0;
        const tx = targetNode.x || 0;
        const ty = targetNode.y || 0;
        return `M ${sx} ${sy} L ${tx} ${ty}`;
    }

    getLinkPath(link) {
        console.log(`[GET LINK PATH] Called for ${link.source}→${link.target}`);
        // Get source and target nodes (use visual redirect if available)
        const visualSourceId = link.visualSource || link.source;
        const visualTargetId = link.visualTarget || link.target;
        const sourceNode = this.visualizer.nodes.find(n => n.id === visualSourceId);
        const targetNode = this.visualizer.nodes.find(n => n.id === visualTargetId);

        if (!sourceNode || !targetNode) {
            console.log(`[GET LINK PATH] Source or target node not found`);
            return 'M 0 0';
        }

        // **TWO-PASS: No need for analyzeLinkConnections(), optimizer uses link.routing**
        const connections = null;

        // If either node is being dragged, use dynamic orthogonal path recalculation
        if (sourceNode.isDragging || targetNode.isDragging) {
            // Create ORTHOGONAL path with direction-aware boundary snapping
            return this.visualizer.createOrthogonalPath(sourceNode, targetNode, link, connections);
        }

        // Use ELK edge routing only if available (only during initial ELK layout)
        if (link.elkSections && link.elkSections.length > 0) {
            const section = link.elkSections[0];

            // Calculate boundary points for start and end
            let startPoint, endPoint;

            if (section.bendPoints && section.bendPoints.length > 0) {
                // If there are bend points, calculate boundary to first/last bend point
                const firstBend = section.bendPoints[0];
                const lastBend = section.bendPoints[section.bendPoints.length - 1];

                startPoint = this.visualizer.getNodeBoundaryPoint(sourceNode, firstBend.x, firstBend.y, link, true, connections);
                endPoint = this.visualizer.getNodeBoundaryPoint(targetNode, lastBend.x, lastBend.y, link, false, connections);

                // Build path: start boundary → bend points → end boundary
                let path = `M ${startPoint.x} ${startPoint.y}`;
                section.bendPoints.forEach(point => {
                    path += ` L ${point.x} ${point.y}`;
                });
                path += ` L ${endPoint.x} ${endPoint.y}`;

                return path;
            } else {
                // No bend points, direct line with boundary calculation
                const sx = sourceNode.x || 0;
                const sy = sourceNode.y || 0;
                const tx = targetNode.x || 0;
                const ty = targetNode.y || 0;

                startPoint = this.visualizer.getNodeBoundaryPoint(sourceNode, tx, ty, link, true, connections);
                endPoint = this.visualizer.getNodeBoundaryPoint(targetNode, sx, sy, link, false, connections);

                return `M ${startPoint.x} ${startPoint.y} L ${endPoint.x} ${endPoint.y}`;
            }
        }

        // Fallback to orthogonal path (after ELK routing is invalidated)
        // This ensures all paths use routing for consistent routing
        return this.visualizer.createOrthogonalPath(sourceNode, targetNode, link, connections);
    }
}
