// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Main entry point for SCXML Interactive Visualizer
 *
 * Handles:
 * - WASM module loading
 * - SCXML source loading (W3C test / Custom / URL)
 * - UI initialization
 * - Error handling
 */

/**
 * Parse URL hash parameters
 * @returns {Object} Parsed parameters
 */
function parseHashParams() {
    const hash = window.location.hash.substring(1); // Remove #
    const params = new URLSearchParams(hash);

    return {
        test: params.get('test'),           // W3C test ID
        scxml: params.get('scxml'),         // Base64 encoded SCXML
        url: params.get('url')              // External SCXML URL
    };
}

/**
 * Load SCXML content from various sources
 * @param {Object} params - URL parameters
 * @returns {Promise<string>} SCXML content
 */
async function loadSCXML(params) {
    // Mode 1: W3C Test (fetch from server)
    if (params.test) {
        const testId = params.test;
        const url = `../../resources/${testId}/test${testId}.scxml`;

        console.log(`📥 Loading W3C test ${testId} from ${url}`);

        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`Failed to fetch test ${testId}: ${response.statusText}`);
        }

        return await response.text();
    }

    // Mode 2: Custom SCXML (base64 from URL fragment)
    if (params.scxml) {
        console.log('📥 Loading SCXML from URL fragment');

        try {
            // URL-decode first, then base64-decode
            const urlDecoded = decodeURIComponent(params.scxml);
            const content = atob(urlDecoded);
            console.log(`✅ Decoded ${content.length} bytes of SCXML`);
            return content;
        } catch (error) {
            throw new Error(`Failed to decode SCXML from URL: ${error.message}`);
        }
    }

    // Mode 3: External URL
    if (params.url) {
        console.log(`📥 Loading SCXML from external URL: ${params.url}`);

        const response = await fetch(params.url);
        if (!response.ok) {
            throw new Error(`Failed to fetch SCXML from ${params.url}: ${response.statusText}`);
        }

        return await response.text();
    }

    throw new Error('No SCXML source specified. Use #test=144, #scxml=<base64>, or #url=<url>');
}

/**
 * Initialize visualizer with SCXML content
 * @param {string} scxmlContent - SCXML content string
 */
async function initVisualizer(scxmlContent) {
    console.log('🎨 Initializing visualizer...');

    // Show loading state
    showLoading(true);

    try {
        // Load WASM module
        console.log('📦 Loading WASM module...');
        const Module = await createVisualizer();

        console.log('✅ WASM module loaded');

        // Create interactive test runner
        const runner = new Module.InteractiveTestRunner();

        // Load SCXML (false = content string, not file path)
        const loadSuccess = runner.loadSCXML(scxmlContent, false);

        if (!loadSuccess) {
            throw new Error('Failed to load SCXML content');
        }

        console.log(`✅ SCXML loaded (${scxmlContent.length} bytes)`);

        // Initialize state machine
        const initSuccess = runner.initialize();

        if (!initSuccess) {
            throw new Error('Failed to initialize state machine');
        }

        console.log('✅ State machine initialized');

        // Get SCXML structure for visualization
        const structure = runner.getSCXMLStructure();
        console.log('📊 SCXML structure:', structure);

        // Log states details
        console.log('  - States count:', structure.states ? structure.states.length : 0);
        if (structure.states) {
            for (let i = 0; i < structure.states.length; i++) {
                console.log(`    State[${i}]:`, structure.states[i]);
            }
        }

        // Log transitions details
        console.log('  - Transitions count:', structure.transitions ? structure.transitions.length : 0);
        if (structure.transitions) {
            for (let i = 0; i < structure.transitions.length; i++) {
                console.log(`    Transition[${i}]:`, structure.transitions[i]);
            }
        }

        console.log('  - Initial:', structure.initial);

        // Extract unique event names from transitions
        const eventNames = new Set();
        if (structure.transitions) {
            structure.transitions.forEach(trans => {
                if (trans.event && trans.event.trim() !== '') {
                    eventNames.add(trans.event);
                }
            });
        }
        const events = Array.from(eventNames).sort();
        console.log('📨 Available events:', events);

        // Create visualizer
        const visualizer = new SCXMLVisualizer('state-diagram', structure);

        // Create execution controller with event list
        const controller = new ExecutionController(runner, visualizer, events);

        // Initial state render
        await controller.updateState();

        console.log('✅ Visualizer ready!');

        // Hide loading state
        showLoading(false);

        // Update test ID in header
        const params = parseHashParams();
        if (params.test) {
            const testIdElement = document.getElementById('test-id');
            if (testIdElement) {
                testIdElement.textContent = params.test;
            }
        }

    } catch (error) {
        console.error('❌ Initialization error:', error);
        showError(error.message);
        showLoading(false);
    }
}

/**
 * Show/hide loading state
 * @param {boolean} show - Show loading if true
 */
function showLoading(show) {
    const loadingElement = document.getElementById('loading-state');
    const mainContent = document.getElementById('main-content');

    if (loadingElement) {
        loadingElement.style.display = show ? 'block' : 'none';
    }

    if (mainContent) {
        mainContent.style.display = show ? 'none' : 'block';
    }
}

/**
 * Show error message
 * @param {string} message - Error message
 */
function showError(message) {
    const errorContainer = document.getElementById('error-container');

    if (errorContainer) {
        errorContainer.innerHTML = `
            <div class="error-message">
                <strong>Error:</strong> ${message}
                <br><br>
                <small>Check browser console for details.</small>
            </div>
        `;
        errorContainer.style.display = 'block';
    }

    // Also hide main content
    const mainContent = document.getElementById('main-content');
    if (mainContent) {
        mainContent.style.display = 'none';
    }
}

/**
 * Load W3C spec references database
 * @returns {Promise<Object>} Spec references
 */
async function loadSpecReferences() {
    try {
        const response = await fetch('spec_references.json');
        if (response.ok) {
            return await response.json();
        }
    } catch (error) {
        console.warn('Failed to load spec references:', error);
    }
    return {};
}

/**
 * Global spec references (loaded on startup)
 * Exposed to window for access by execution-controller
 */
window.specReferences = {};

/**
 * Main initialization on page load
 */
window.addEventListener('load', async () => {
    console.log('🚀 SCXML Interactive Visualizer starting...');

    try {
        // Load spec references database
        window.specReferences = await loadSpecReferences();
        console.log('📚 Spec references loaded:', Object.keys(window.specReferences).length, 'tests');

        // Parse URL parameters
        const params = parseHashParams();

        console.log('📋 URL parameters:', params);

        // Load SCXML content
        const scxmlContent = await loadSCXML(params);

        // Initialize visualizer
        await initVisualizer(scxmlContent);

    } catch (error) {
        console.error('❌ Fatal error:', error);
        showError(error.message);
    }
});

/**
 * Handle hash change (for navigation)
 */
window.addEventListener('hashchange', () => {
    console.log('🔄 Hash changed, reloading...');
    window.location.reload();
});
