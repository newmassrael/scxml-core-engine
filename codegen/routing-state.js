// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * RoutingState
 *
 * Unified routing state for a transition link.
 * Replaces the legacy confirmedDirections object with a single source
 * of truth for transition routing information.
 *
 * This eliminates state synchronization issues and provides a clear API
 * for accessing routing information.
 */
class RoutingState {
    constructor() {
        // Edge assignments
        this.sourceEdge = null;       // 'top'|'bottom'|'left'|'right'
        this.targetEdge = null;       // 'top'|'bottom'|'left'|'right'

        // Snap point coordinates
        this.sourcePoint = null;      // {x: number, y: number}
        this.targetPoint = null;      // {x: number, y: number}

        // Path characteristics
        this.pathType = null;         // 'direct'|'z-path'|'l-path'|'complex'
        this.midY = null;             // For z-paths: y-coordinate of horizontal segment
    }

    /**
     * Get outgoing direction for source edge
     * @returns {string|null} Direction string like 'to-top', 'to-bottom', etc.
     */
    get sourceDirection() {
        return this.sourceEdge ? EdgeDirection.toOutgoingDirection(this.sourceEdge) : null;
    }

    /**
     * Get incoming direction for target edge
     * @returns {string|null} Direction string like 'from-top', 'from-bottom', etc.
     */
    get targetDirection() {
        return this.targetEdge ? EdgeDirection.toIncomingDirection(this.targetEdge) : null;
    }

    /**
     * Check if routing state is valid (has all required fields)
     * @returns {boolean} True if state is valid
     */
    get isValid() {
        return !!(
            this.sourceEdge &&
            this.targetEdge &&
            this.sourcePoint &&
            this.targetPoint &&
            this.sourcePoint.x !== undefined &&
            this.sourcePoint.y !== undefined &&
            this.targetPoint.x !== undefined &&
            this.targetPoint.y !== undefined
        );
    }

    /**
     * Check if source and target edges are both vertical
     * @returns {boolean} True if both vertical
     */
    get isBothVertical() {
        return this.sourceEdge && this.targetEdge &&
               EdgeDirection.isVertical(this.sourceEdge) &&
               EdgeDirection.isVertical(this.targetEdge);
    }

    /**
     * Check if source and target edges are both horizontal
     * @returns {boolean} True if both horizontal
     */
    get isBothHorizontal() {
        return this.sourceEdge && this.targetEdge &&
               EdgeDirection.isHorizontal(this.sourceEdge) &&
               EdgeDirection.isHorizontal(this.targetEdge);
    }

    /**
     * Check if edges are mixed (one vertical, one horizontal)
     * @returns {boolean} True if mixed
     */
    get isMixed() {
        return this.sourceEdge && this.targetEdge &&
               EdgeDirection.isVertical(this.sourceEdge) !== EdgeDirection.isVertical(this.targetEdge);
    }

    /**
     * Create RoutingState from edge assignments
     * @param {string} sourceEdge - Source edge name
     * @param {string} targetEdge - Target edge name
     * @returns {RoutingState} New routing state
     */
    static fromEdges(sourceEdge, targetEdge) {
        const state = new RoutingState();
        state.sourceEdge = sourceEdge;
        state.targetEdge = targetEdge;
        return state;
    }

    /**
     * Update snap points
     * @param {Object} sourcePoint - Source snap point {x, y}
     * @param {Object} targetPoint - Target snap point {x, y}
     */
    setPoints(sourcePoint, targetPoint) {
        this.sourcePoint = sourcePoint;
        this.targetPoint = targetPoint;
    }

    /**
     * Update edges
     * @param {string} sourceEdge - Source edge name
     * @param {string} targetEdge - Target edge name
     */
    setEdges(sourceEdge, targetEdge) {
        this.sourceEdge = sourceEdge;
        this.targetEdge = targetEdge;
    }

    /**
     * Create a copy of this routing state
     * @returns {RoutingState} Copy of this state
     */
    clone() {
        const copy = new RoutingState();
        copy.sourceEdge = this.sourceEdge;
        copy.targetEdge = this.targetEdge;
        copy.sourcePoint = this.sourcePoint ? { ...this.sourcePoint } : null;
        copy.targetPoint = this.targetPoint ? { ...this.targetPoint } : null;
        copy.pathType = this.pathType;
        copy.midY = this.midY;
        return copy;
    }

    /**
     * Serialize to JSON
     * @returns {Object} JSON representation
     */
    toJSON() {
        return {
            sourceEdge: this.sourceEdge,
            targetEdge: this.targetEdge,
            sourcePoint: this.sourcePoint,
            targetPoint: this.targetPoint,
            pathType: this.pathType,
            midY: this.midY
        };
    }

    /**
     * Create from JSON
     * @param {Object} json - JSON representation
     * @returns {RoutingState} New routing state
     */
    static fromJSON(json) {
        const state = new RoutingState();
        state.sourceEdge = json.sourceEdge;
        state.targetEdge = json.targetEdge;
        state.sourcePoint = json.sourcePoint;
        state.targetPoint = json.targetPoint;
        state.pathType = json.pathType;
        state.midY = json.midY;
        return state;
    }

    /**
     * String representation for debugging
     * @returns {string} String representation
     */
    toString() {
        return `RoutingState(${this.sourceEdge}→${this.targetEdge}, ` +
               `points: ${this.sourcePoint ? 'set' : 'null'}/${this.targetPoint ? 'set' : 'null'}, ` +
               `valid: ${this.isValid})`;
    }
}
