// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
 * The id an ARROW carries — not an identity for a transition.
 *
 * ⚠ This used to say "Generate unique transition ID" and "W3C SCXML
 * Compliance: Transitions are uniquely identified by source state, event, and
 * target state". The second sentence is false, and the first follows it into
 * being false. W3C SCXML lets two transitions share a source, an event and a
 * target and differ only by their `cond`; that is the ordinary way to write a
 * branch. Measured over this repository's documents, 33 of 1557 ids name more
 * than one transition, across 27 documents, and one names THIRTEEN —
 * `report_eventless_done`, thirteen guarded alternatives to the same state.
 *
 * ⚠⚠ It cannot be made unique here, and that is the more useful half to know.
 * The engine reports `{source, target, event}` for the transition it took
 * (`InteractiveTestRunner::getLastTransition`) and nothing that separates one
 * guard from another, so an id the diagram could tell apart is an id the
 * execution could never produce. Narrowing this function without changing
 * what the engine reports would take highlighting from "marks all thirteen"
 * to "marks none".
 *
 * What it IS good for: naming the arrow. Transitions sharing a pair of states
 * are drawn as one arrow anyway (`LinkBuilder.mergeParallelTransitions`), so
 * an id that collides exactly where the drawing merges is the right key for
 * the drawn element. Callers that need to tell two transitions apart in a
 * LIST have `data-transition-index`, which is unique by construction.
 *
 * Single Source of Truth for transition identification used by:
 * - renderer.js: SVG link data-transition-id attributes
 * - focus-manager.js: Highlight and focus operations
 * - interaction-handler.js: Transition list panel items
 * - control-handler.js: Panel highlighting
 *
 * Format: source_event_target (e.g., "s02_fail_fail", "s01_eventless_s02")
 * W3C SCXML 5.9.2: Targetless transitions use source as target (self-loop)
 *
 * @param {Object} transition - Transition object with source, event, target properties
 * @returns {string|null} The arrow's id, shared by every transition drawn on
 *   it, or null if the transition is invalid
 */
function getTransitionId(transition) {
    if (!transition || !transition.source) {
        return null;
    }
    // W3C SCXML 5.9.2: Targetless transitions (no target attribute) use source as target (self-loop)
    const target = transition.target || transition.source;
    // Use event name for unique identification, fallback to 'eventless' for eventless transitions
    const eventName = transition.event || 'eventless';
    return `${transition.source}_${eventName}_${target}`;
}
