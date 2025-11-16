// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * EdgeDirection
 *
 * Utility class for edge/direction conversions in transition routing.
 * Centralizes conversion logic between edge names ('top', 'bottom', 'left', 'right')
 * and direction strings ('to-top', 'from-top', etc.).
 */
class EdgeDirection {
    static EDGES = ['top', 'bottom', 'left', 'right'];

    /**
     * Convert edge name to outgoing direction
     * @param {string} edge - Edge name ('top', 'bottom', 'left', 'right')
     * @returns {string} Outgoing direction ('to-top', 'to-bottom', etc.)
     */
    static toOutgoingDirection(edge) {
        return 'to-' + edge;
    }

    /**
     * Convert edge name to incoming direction
     * @param {string} edge - Edge name ('top', 'bottom', 'left', 'right')
     * @returns {string} Incoming direction ('from-top', 'from-bottom', etc.)
     */
    static toIncomingDirection(edge) {
        return 'from-' + edge;
    }

    /**
     * Extract edge name from direction string
     * @param {string} direction - Direction string ('to-top', 'from-bottom', etc.)
     * @returns {string} Edge name ('top', 'bottom', 'left', 'right')
     */
    static fromDirection(direction) {
        return direction.replace(/^(to-|from-)/, '');
    }

    /**
     * Check if edge or direction is vertical
     * @param {string} edgeOrDirection - Edge name or direction string
     * @returns {boolean} True if vertical (top/bottom)
     */
    static isVertical(edgeOrDirection) {
        const edge = this.fromDirection(edgeOrDirection);
        return edge === 'top' || edge === 'bottom';
    }

    /**
     * Check if edge or direction is horizontal
     * @param {string} edgeOrDirection - Edge name or direction string
     * @returns {boolean} True if horizontal (left/right)
     */
    static isHorizontal(edgeOrDirection) {
        return !this.isVertical(edgeOrDirection);
    }

    /**
     * Get opposite edge
     * @param {string} edge - Edge name
     * @returns {string} Opposite edge name
     */
    static opposite(edge) {
        const opposites = {
            'top': 'bottom',
            'bottom': 'top',
            'left': 'right',
            'right': 'left'
        };
        return opposites[edge] || edge;
    }
}
