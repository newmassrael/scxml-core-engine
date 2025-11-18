// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Shared utility functions for SCXML Interactive Visualizer
 * DRY Principle: Common logic extracted from multiple files
 *
 * Global functions accessible from all scripts
 */

/**
 * Get resources path prefix based on environment detection
 * Shared logic between main.js and execution-controller.js
 *
 * @returns {string} Resources path prefix
 */
function getResourcesPath() {
    // Environment detection
    const isGitHubPages = window.location.hostname.includes('github.io');

    // Path resolution (Single Source of Truth):
    // - GitHub Pages: resources/ (visualizer/ → visualizer/resources/)
    // - Local: ../resources (visualizer/ → resources/)
    if (isGitHubPages) {
        return 'resources/';  // GitHub Pages: same directory
    }
    return '../resources';  // Local: visualizer/ → resources/
}

/**
 * Get environment name for logging
 *
 * @returns {string} Environment name
 */
function getEnvironmentName() {
    const isGitHubPages = window.location.hostname.includes('github.io');
    const isLocalhost = window.location.hostname === 'localhost' ||
                       window.location.hostname === '127.0.0.1' ||
                       window.location.hostname === '';

    return isGitHubPages ? 'GitHub Pages' :
           isLocalhost ? 'Localhost' :
           'Local Dev';
}

/**
 * Generate unique transition ID for consistent identification across all components
 * W3C SCXML Compliance: Transitions are uniquely identified by source state, event, and target state
 *
 * Single Source of Truth for transition identification used by:
 * - renderer.js: SVG link data-transition-id attributes
 * - focus-manager.js: Highlight and focus operations
 * - interaction-handler.js: Transition list panel items
 * - control-handler.js: Panel highlighting
 *
 * Format: source_event_target (e.g., "s02_fail_fail", "s01_eventless_s02")
 *
 * @param {Object} transition - Transition object with source, event, target properties
 * @returns {string|null} Unique transition ID or null if transition is invalid
 */
function getTransitionId(transition) {
    if (!transition || !transition.source || !transition.target) {
        return null;
    }
    // Use event name for unique identification, fallback to 'eventless' for eventless transitions
    const eventName = transition.event || 'eventless';
    return `${transition.source}_${eventName}_${transition.target}`;
}
