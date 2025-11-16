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
    const isToolsWeb = window.location.pathname.includes('/tools/web/');

    // Path resolution (Single Source of Truth):
    // - GitHub Pages: resources/ (visualizer/ → visualizer/resources/)
    // - Deployed localhost: ../resources (visualizer/ → resources/)
    // - Development (tools/web/): ../../resources (tools/web/ → resources/)
    if (isGitHubPages) {
        return 'resources/';  // GitHub Pages: same directory
    }
    if (isToolsWeb) {
        return '../../resources';  // Development: tools/web/visualizer.html
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
