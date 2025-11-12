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
    const isGitHubPages = window.location.hostname.includes('github.io');
    const isLocalhost = window.location.hostname === 'localhost' ||
                       window.location.hostname === '127.0.0.1' ||
                       window.location.hostname === '';

    // Path resolution:
    // - GitHub Pages: ../resources (visualizer at test-results/visualizer/, resources at test-results/resources/)
    // - Localhost: resources (symlink in tools/web/ → ../../resources)
    // - Project root server: ../../resources (fallback)
    return isGitHubPages ? '../resources' :
           isLocalhost ? 'resources' :
           '../../resources';
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
