// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Path Utils - Handles path intersection and collision detection
 */

class PathUtils {
    constructor(optimizer) {
        this.optimizer = optimizer;
    }

    calculatePathIntersections(path1, path2) {
        const segments1 = this.optimizer.getPathSegments(path1);
        const segments2 = this.optimizer.getPathSegments(path2);

        let intersections = 0;

        segments1.forEach(seg1 => {
            segments2.forEach(seg2 => {
                if (this.optimizer.segmentsIntersect(seg1, seg2)) {
                    intersections++;
                }
            });
        });

        return intersections;
    }

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

    pathIntersectsNode(path, node, options = {}) {
        const { skipFirstSegment = false, skipLastSegment = false } = options;
        const { halfWidth, halfHeight } = TransitionLayoutOptimizer.getNodeSize(node);

        const nodeLeft = node.x - halfWidth;
        const nodeRight = node.x + halfWidth;
        const nodeTop = node.y - halfHeight;
        const nodeBottom = node.y + halfHeight;

        const segments = this.optimizer.getPathSegments(path);

        for (let i = 0; i < segments.length; i++) {
            const segment = segments[i];
            const isFirstSegment = i === 0;
            const isLastSegment = i === segments.length - 1;
            const isDirectLine = segments.length === 1;

            // Special handling for direct line (single segment)
            if (isDirectLine && (skipFirstSegment || skipLastSegment)) {
                const excludeStart = skipFirstSegment;
                const excludeEnd = skipLastSegment;
                
                if (this.optimizer.segmentIntersectsRectExcludingPoints(
                    segment, nodeLeft, nodeTop, nodeRight, nodeBottom, excludeStart, excludeEnd
                )) {
                    return true;
                }
                continue; // Skip normal check
            }

            // Skip first segment if requested (for source node collision check)
            if (skipFirstSegment && isFirstSegment) {
                continue;
            }

            // Skip last segment if requested (for target node collision check)
            if (skipLastSegment && isLastSegment) {
                continue;
            }

            // Check if segment intersects with node bounding box
            if (this.optimizer.segmentIntersectsRect(segment, nodeLeft, nodeTop, nodeRight, nodeBottom)) {
                return true;
            }
        }

        return false;
    }

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
            if (this.optimizer.segmentsIntersect(segment, rectSeg)) {
                return true;
            }
        }

        return false;
    }

    /**
     * Check if segment intersects rectangle, optionally excluding start/end points
     * @param {Object} segment - {x1, y1, x2, y2}
     * @param {number} left - Rectangle left edge
     * @param {number} top - Rectangle top edge
     * @param {number} right - Rectangle right edge
     * @param {number} bottom - Rectangle bottom edge
     * @param {boolean} excludeStart - Exclude startpoint area (MIN_SEGMENT distance)
     * @param {boolean} excludeEnd - Exclude endpoint area (MIN_SEGMENT distance)
     * @returns {boolean} True if (shortened) segment intersects rectangle
     */
    segmentIntersectsRectExcludingPoints(segment, left, top, right, bottom, excludeStart = false, excludeEnd = false) {
        // Early return: no exclusion needed
        if (!excludeStart && !excludeEnd) {
            return this.optimizer.segmentIntersectsRect(segment, left, top, right, bottom);
        }

        const MIN_SEGMENT = TransitionLayoutOptimizer.MIN_SEGMENT_LENGTH;
        let { x1, y1, x2, y2 } = segment;

        const isVertical = Math.abs(x2 - x1) < 1;
        const isHorizontal = Math.abs(y2 - y1) < 1;

        // Only orthogonal segments are supported (W3C SCXML transitions are always orthogonal)
        if (!isVertical && !isHorizontal) {
            console.warn('[PATH-UTILS] Non-orthogonal segment detected, using full segment for collision check');
            return this.optimizer.segmentIntersectsRect(segment, left, top, right, bottom);
        }

        // Calculate segment length
        const segmentLength = isVertical ? Math.abs(y2 - y1) : Math.abs(x2 - x1);
        
        // If segment is too short to exclude both ends, check full segment
        if (excludeStart && excludeEnd && segmentLength <= 2 * MIN_SEGMENT) {
            console.warn(`[PATH-UTILS] Segment too short (${segmentLength.toFixed(1)}px) to exclude both endpoints, using full segment`);
            return this.optimizer.segmentIntersectsRect(segment, left, top, right, bottom);
        }

        // Shorten segment from start if requested
        if (excludeStart) {
            if (isVertical) {
                y1 = y2 > y1 ? y1 + MIN_SEGMENT : y1 - MIN_SEGMENT;
            } else { // isHorizontal
                x1 = x2 > x1 ? x1 + MIN_SEGMENT : x1 - MIN_SEGMENT;
            }
        }

        // Shorten segment from end if requested
        if (excludeEnd) {
            if (isVertical) {
                y2 = y2 > y1 ? y2 - MIN_SEGMENT : y2 + MIN_SEGMENT;
            } else { // isHorizontal
                x2 = x2 > x1 ? x2 - MIN_SEGMENT : x2 + MIN_SEGMENT;
            }
        }

        // Reuse segment object to avoid allocation
        const shortenedSegment = { x1, y1, x2, y2 };
        return this.optimizer.segmentIntersectsRect(shortenedSegment, left, top, right, bottom);
    }

    evaluateCombination(link, sourceNode, targetNode, sourceEdge, targetEdge, assignedPaths, nodes) {
        
        const sourcePoint = this.optimizer.getEdgeCenterPoint(sourceNode, sourceEdge);
        const targetPoint = this.optimizer.getEdgeCenterPoint(targetNode, targetEdge);

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
}
